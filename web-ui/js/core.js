(function(window, document, E) {
    function Canvas(elem) {
        this.elem = elem;
        this.view = E('div', {
            className: 'view',
            parent: this.elem,
        });

        this.pos = {x: 0, y: 0};
        this.screens = [];
        this.names = ['Main Laptop', 'Desktop', 'Secondary Laptop', 'Gallifrey', 'Kronos', 'Atlantis'];
        this.nameIndex = 0;
        this.focuses = {};
        
        // Setup mouse events
        function mouseEvent(cb, e) {
            cb.call(this, {
                id: null,
                target: e.target,
                pos: {x: e.clientX, y: e.clientY},
                ctrlKey: e.ctrlKey,
            });
        }

        elem.addEventListener('mousedown', mouseEvent.bind(this, this.dragStart), false);
        window.addEventListener('mousemove', mouseEvent.bind(this, this.dragMove), false);        
        window.addEventListener('mouseup', mouseEvent.bind(this, this.dragEnd), false);

        // Setup touch events
        function touchEvent(cb, e) {
            for (var i = 0; i < e.changedTouches.length; i++) {
                var touch = e.changedTouches[i];
                cb.call(this, {
                    id: touch.identifier,
                    target: e.target,
                    pos: {x: touch.clientX, y: touch.clientY},
                    ctrlKey: e.ctrlKey,
                });
            }
            
            e.preventDefault();
        }
        
        elem.addEventListener('touchstart', touchEvent.bind(this, this.dragStart), false);
        elem.addEventListener('touchmove', touchEvent.bind(this, this.dragMove), false);
        elem.addEventListener('touchend', touchEvent.bind(this, this.dragEnd), false);

        // Setup resize event
        // window.addEventListener('resize', this.recenter.bind(this), false);
    }

    Canvas.prototype.getCenter = function() {
        var rect = this.elem.getBoundingClientRect();
        return {
            x: rect.width / 2,
            y: rect.height / 2,
        }
    };

    Canvas.prototype.localizePoint = function(pos) {
        var rect = this.elem.getBoundingClientRect();
        return {
            x: pos.x - rect.left - this.pos.x,
            y: pos.y - rect.top - this.pos.y,
        };
    };    

    Canvas.prototype.addScreen = function(pos) {
        var id = this.screens.length;
        var screen = new Screen({
            id: id,
            name: this.names[this.nameIndex],
            pos: pos,
            size: {x: 160, y: 100},
        });

        this.nameIndex = (this.nameIndex + 1) % this.names.length;
        screen.connectClosest(this.screens);
        this.screens.push(screen);
        this.view.appendChild(screen.elem);
        return screen;
    };

    Canvas.prototype.setPos = function(pos) {
        this.pos = pos;
        this.view.style.transform = 'translate(' + this.pos.x + 'px,' + this.pos.y + 'px)';
    };

    Canvas.prototype.move = function(d) {
        this.setPos({
            x: this.pos.x + d.x,
            y: this.pos.y + d.y
        });
    };

    Canvas.prototype.recenter = function() {
        this.setPos({x: 0, y: 0});
    };   
    
    Canvas.prototype.dragStart = function(e) {
        // Add new screens when ctrl is pressed
        if (e.ctrlKey) {
            this.addScreen(this.localizePoint(e.pos));
            return;
        }
        
        if (this.focuses[e.id]) {
            return;
        }

        // Bubble target until reached a valid target or the canvas
        var target = e.target;
        while (target != this.elem && !target.dataset.id) {
            target = target.parentElement;
        }

        // Obtain draggable interface for target
        var dragTarget;
        if (target != this.elem) {
            dragTarget = this.screens[target.dataset.id];
            this.view.appendChild(dragTarget.elem);
        } else {
            dragTarget = this;
        }

        dragTarget.elem.classList.add('dragging');
        this.focuses[e.id] = {
            target: dragTarget,
            lastPos: e.pos,
        };
    };

    Canvas.prototype.dragMove = function(e) {
        let focus = this.focuses[e.id];
        if (!focus) {
            return;
        }

        focus.target.move({
            x: e.pos.x - focus.lastPos.x,
            y: e.pos.y - focus.lastPos.y,
        });
        
        focus.lastPos = e.pos;
    };

    Canvas.prototype.dragEnd = function(e) {
        let focus = this.focuses[e.id];
        if (!focus) {
            return;
        }

        var target = focus.target;
        if (target.connectClosest) {
            let blarg = this.screens.filter((screen) => {
                for (key in this.focuses) {
                    if (screen == this.focuses[key].target) {
                        return false;
                    }
                }
                return true;
            });
            target.connectClosest(blarg);
        }
        focus.target.elem.classList.remove('dragging');
        delete this.focuses[e.id];
    };   

    function Screen(params) {
        this.elem = E('div', {
            dataset: { id: params.id },
            className: 'screen draggable',
            children: [E('h3', {
                className: 'screen-name',
                textContent: params.name,
            })]
        });

        this.setSize(params.size);
        this.setPos(params.pos);
        this.edges = {
            top: null,
            right: null,
            bottom: null,
            left: null
        };
    }

    Screen.prototype.setPos = function(pos) {
        this.pos = pos;
        this.elem.style.left = this.pos.x - this.size.x / 2 + 'px';
        this.elem.style.top = this.pos.y - this.size.y / 2 + 'px';
    };

    Screen.prototype.setSize = function(size) {
        this.size = size;
        this.elem.style.width = this.size.x + 'px';
        this.elem.style.height = this.size.y + 'px';
    };

    Screen.prototype.move = function(d) {
        this.setPos({
            x: this.pos.x + d.x,
            y: this.pos.y + d.y,
        });
    };

    Screen.prototype.distance = function(pos) {
        var dx = this.pos.x - pos.x;
        var dy = this.pos.y - pos.y;
        return Math.sqrt(dx * dx + dy * dy);
    };

    Screen.prototype.closest = function(screens) {
        var pos = this.pos;
        return screens.reduce((min, curr) => {
            var dist = curr.distance(pos);
            if (!min || dist < min.dist) {
                return {dist: dist, screen: curr};
            }
            return min;
        }, null);
    };

    Screen.prototype.connect = function(screen) {
        var pad = 5;
        
        // Compute edge-edge deltas
        var dx = this.pos.x - screen.pos.x;
        var dy = this.pos.y - screen.pos.y;
        
        if (Math.abs(dx) > Math.abs(dy)) {
            if (dx > 0) {
                screen.edges.right = this;
                this.edges.left = screen;
                this.setPos({x: screen.pos.x + screen.size.x + pad, y: screen.pos.y});
            } else {
                screen.edges.left = this;
                this.edges.right = screen;
                this.setPos({x: screen.pos.x - this.size.x - pad, y: screen.pos.y});
            }
        } else {
            if (dy > 0) {
                screen.edges.bottom = this;
                this.edges.top = screen;
                this.setPos({x: screen.pos.x, y: screen.pos.y + screen.size.y + pad});
            } else {
                screen.edges.top = this;
                this.edges.bottom = screen;
                this.setPos({x: screen.pos.x, y: screen.pos.y - this.size.y - pad});
            }
        }
    };

    Screen.prototype.connectClosest = function(screens) {
        let closest = this.closest(screens);
        if (closest) this.connect(closest.screen);
    };
    
    new Canvas(document.querySelector('.canvas'));
})(window, document, element.html)
