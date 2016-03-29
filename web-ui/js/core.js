(function(window, document, E) {
    var gridPad = 5;

    function vectorAdd(a, b) {
        return a.map((val, dim) => val + b[dim]);
    }

    function vectorSub(a, b) {
        return a.map((val, dim) => val - b[dim]);
    }

    function vectorDistSquare(a, b) {
        return vectorSub(a, b).reduce((acc, val) => {
            return acc + (val * val);
        }, 0);
    }

    function Canvas(elem, socket) {
        this.elem = elem;
        this.view = E('div', {
            className: 'view',
            parent: this.elem,
        });

        this.pos = [0, 0];
        this.screens = [];
        this.focuses = {};

        function dragStart(e) {
            // Add dummy screens when ctrl is pressed
            if (e.ctrlKey) {
                this.addDummyScreen(this.localizePos(e.pos));
                return;
            }

            // Ignore if identifier is already focused on an item
            if (this.focuses[e.id]) {
                return;
            }

            // Bubble target until reached a valid target or the canvas
            var elem = e.target;
            while (elem != this.elem && !elem.dataset.id) {
                elem = elem.parentElement;
            }

            // Obtain draggable interface for target
            var target;
            if (elem != this.elem) {
                target = this.screens[elem.dataset.id];
                this.view.appendChild(target.elem);
            } else {
                target = this;
            }

            // Prevent multiple identifiers from dragging a single target
            for (var key in this.focuses) {
                if (this.focuses[key].target == target) {
                    return;
                }
            }

            // Call target callback
            target.elem.classList.add('dragging');
            if (target.dragStart) {
                e.source = this;
                e.delta = [0, 0];
                target.dragStart(e);
            }

            this.focuses[e.id] = {
                target: target,
                lastPos: e.pos,
            };
        };

        function dragMove(e) {
            var focus = this.focuses[e.id];
            if (!focus) {
                return;
            }

            // Call target callback
            var target = focus.target;
            if (target.dragMove) {
                e.source = this;
                e.delta = vectorSub(e.pos, focus.lastPos);
                target.dragMove(e);
            }

            focus.lastPos = e.pos;
        };

        function dragEnd(e) {
            var focus = this.focuses[e.id];
            if (!focus) {
                return;
            }

            // Call target callback
            var target = focus.target;
            target.elem.classList.remove('dragging');
            if (target.dragEnd) {
                e.source = this;
                e.delta = vectorSub(e.pos, focus.lastPos);
                target.dragEnd(e);
            }

            // Update the cluster
            socket.send(JSON.stringify({
                Screens: this.cluster.screens.map((screen, id) => {
                    if (this.screens[id]) {
                        // Update the edges
                        var edges = this.screens[id].edges;
                        screen.edges = {
                            left: edges[0][0] ? edges[0][0].id : null,
                            right: edges[0][1] ? edges[0][1].id : null,
                            top: edges[1][0] ? edges[1][0].id : null,
                            bottom: edges[1][1] ? edges[1][1].id : null,
                        };
                    }

                    return screen;
                }),
            }));

            delete this.focuses[e.id];
        };

        // Setup mouse events
        function mouseEvent(cb, e) {
            if (e.button !== 0) return;
            cb.call(this, {
                id: null,
                target: e.target,
                pos: [e.clientX, e.clientY],
                ctrlKey: e.ctrlKey,
                altKey: e.altKey,
                shiftKey: e.shiftKey,
            });
        }

        elem.addEventListener('mousedown', mouseEvent.bind(this, dragStart), false);
        window.addEventListener('mousemove', mouseEvent.bind(this, dragMove), false);
        window.addEventListener('mouseup', mouseEvent.bind(this, dragEnd), false);

        // Setup touch events
        function touchEvent(cb, e) {
            for (var i = 0; i < e.changedTouches.length; i++) {
                var touch = e.changedTouches[i];
                cb.call(this, {
                    id: touch.identifier,
                    target: e.target,
                    pos: [touch.clientX, touch.clientY],
                    ctrlKey: e.ctrlKey,
                    altKey: e.altKey,
                    shiftKey: e.shiftKey,
                });
            }

            e.preventDefault();
        }

        elem.addEventListener('touchstart', touchEvent.bind(this, dragStart), false);
        elem.addEventListener('touchmove', touchEvent.bind(this, dragMove), false);
        elem.addEventListener('touchend', touchEvent.bind(this, dragEnd), false);

        // Network events
        socket.onmessage = (e) => {
            var obj = JSON.parse(e.data);
            var type = Object.keys(obj)[0];
            var event = obj[type];
            switch(type) {
            case "Cluster":
                this.replaceCluster(event);
                break;
            }
        };
    }

    Canvas.prototype.setPos = function(pos) {
        this.pos = pos;
        this.view.style.transform = 'translate(' + this.pos[0] + 'px,' + this.pos[1] + 'px)';
    };

    Canvas.prototype.getCenter = function() {
        var rect = this.elem.getBoundingClientRect();
        return [
            rect.width / 2,
            rect.height / 2,
        ];
    };

    Canvas.prototype.localizePos = function(pos) {
        var rect = this.elem.getBoundingClientRect();
        return [
            pos[0] - rect.left - this.pos[0],
            pos[1] - rect.top - this.pos[1],
        ];
    };

    Canvas.prototype.replaceCluster = function(cluster) {
        this.cluster = cluster;
        this.screens.forEach((screen) => {
            screen.elem.parentElement.removeChild(screen.elem)
        });
        this.screens = [];
        this.addScreens(cluster, cluster.local_screen, this.getCenter());
    };

    Canvas.prototype.addScreens = function(cluster, id, pos) {
        var obj = this.screens[id];
        if (obj) return obj;

        var obj = cluster.screens[id];
        if (!obj) return null;

        var screen = new Screen({
            id: id,
            name: obj.name,
            pos: pos,
            size: [200, 125],
            local: cluster.local_screen === id,
        });

        this.screens[id] = screen;
        this.view.appendChild(screen.elem);

        // Resolve edges
        screen.edges = [[
            this.addScreens(cluster, obj.edges.left, vectorSub(pos, [screen.size[0] + gridPad, 0])),
            this.addScreens(cluster, obj.edges.right, vectorAdd(pos, [screen.size[0] + gridPad, 0])),
        ], [
            this.addScreens(cluster, obj.edges.top, vectorSub(pos, [0, screen.size[1] + gridPad])),
            this.addScreens(cluster, obj.edges.bottom, vectorAdd(pos, [0, screen.size[1] + gridPad])),
        ]];

        return screen;
    };

    Canvas.prototype.addDummyScreen = function(pos) {
        var screen = new Screen({
            id: this.screens.length,
            name: "Dummy Screen",
            pos: pos,
            size: [200, 125],
        });

        screen.connectClosest(this.getScreens());
        this.screens.push(screen);
        this.view.appendChild(screen.elem);
        return screen;
    };

    Canvas.prototype.getScreens = function() {
        return this.screens.filter((screen) => {
            // Ignore screens that have focuses on them
            for (var key in this.focuses) {
                if (screen == this.focuses[key].target) {
                    return false;
                }
            }
            return true;
        });
    };

    Canvas.prototype.dragMove = function(e) {
        this.setPos(vectorAdd(this.pos, e.delta));
    }

    function Screen(params) {
        this.id = params.id;

        this.elem = E('div', {
            dataset: { id: params.id },
            className: ['screen', params.local ? 'local' : '', 'draggable'],
            children: [E('h3', {
                className: ['screen-name'],
                textContent: params.name,
            })]
        });

        this.setSize(params.size);
        this.setPos(params.pos);
        this.edges = [[null, null], [null, null]];
    }

    Screen.prototype.setPos = function(pos) {
        this.pos = pos;
        this.elem.style.left = this.pos[0] - this.size[0] / 2 + 'px';
        this.elem.style.top = this.pos[1] - this.size[1] / 2 + 'px';
    };

    Screen.prototype.setSize = function(size) {
        this.size = size;
        this.elem.style.width = this.size[0] + 'px';
        this.elem.style.height = this.size[1] + 'px';
    };

    Screen.prototype.closest = function(screens) {
        // TODO: Probably should take into account different screen sizes
        var pos = this.pos;
        return screens.reduce((min, curr) => {
            var dist = vectorDistSquare(curr.pos, pos);
            if (min == null || dist < min.dist) {
                return {screen: curr, dist: dist};
            }
            return min;
        }, null);
    };

    Screen.prototype.connect = function(other) {
        // Find the dimension with the largest delta
        // TODO: Take into account aspect ratio
        var delta = vectorSub(this.pos, other.pos);
        var dim = delta.reduce((max, curr, dim) => {
            if (max == null || Math.abs(curr) > Math.abs(delta[max])) {
                return dim;
            }
            return max;
        }, null);

        // Determine which sides to connect together
        var newEdges = [[null, null], [null, null]];
        var side = delta[dim] < 0 ? 0 : 1;
        newEdges[dim][1 - side] = other;

        // Walk around graph through adjacent dimensions to find neighbours
        other.edges.forEach((edgeDim, pathDim) => {
            if (pathDim == dim) return;
            edgeDim.forEach((screen, pathSide) => {
                if (!screen) return;
                screen = screen.edges[dim][side];
                if (!screen) return;

                // Found a neighbour
                newEdges[pathDim][pathSide] = screen;

                screen = screen.edges[dim][side];
                if (!screen) return;
                screen = screen.edges[pathDim][1 - pathSide];
                if (!screen) return;

                // Found a neighbour (if found by one path, should be found by all paths)
                newEdges[dim][side] = screen;
            });
        });

        // Remove old connections
        this.edges.forEach((edgeDim, dim) => {
            edgeDim.forEach((screen, side) => {
                if (screen) {
                    screen.edges[dim][1 - side] = null;
                }
            });
        });

        // Create new connections
        this.edges = newEdges;
        this.edges.forEach((edgeDim, dim) => {
            edgeDim.forEach((screen, side) => {
                if (screen) {
                    screen.edges[dim][1 - side] = this;
                }
            });
        });

        // Move this screen adjacent to the other screen
        var pos = other.pos.slice();
        var offset = ((this.size[dim] + other.size[dim]) / 2 + gridPad);
        pos[dim] += (2 * side - 1) * offset;
        this.setPos(pos);
    };

    Screen.prototype.connectClosest = function(screens) {
        var closest = this.closest(screens);
        if (closest) this.connect(closest.screen);
    };

    Screen.prototype.dragMove = function(e) {
        this.setPos(vectorAdd(this.pos, e.delta));
    };

    Screen.prototype.dragEnd = function(e) {
        this.connectClosest(e.source.getScreens());
    };

    var socket = new WebSocket("ws://127.0.0.1:3012");
    socket.onopen = function(e) {
        new Canvas(document.querySelector('.canvas'), socket);
    };

    socket.onerror = function(e) {
        alert("Failed to connect to server");
    };
})(window, document, element.html)
