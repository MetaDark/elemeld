(function(window, document, E) {
    function Canvas(elem) {
        var screens = [];
        var focuses = {};

        function localizePoint(x, y) {
            var rect = elem.getBoundingClientRect();
            console.log(rect);
            return {
                x: x - rect.left,
                y: y - rect.top,
            };
        }
        
        function dragStart(target, id, x, y) {
            if (focuses[id]) {
                return;
            }

            while (target != elem && !target.classList.contains('dragabble')) {
                target = target.parentElement;
            }

            if (target == elem) {
                var rel = localizePoint(x, y);
                var screen = new Screen(rel.x, rel.y);
                screens.push(screen);
                target = screen.elem;
            }
            
            target.classList.add('drag');
            elem.appendChild(target);
            focuses[id] = {
                target: target,
                lastX: x,
                lastY: y,
            };
        }

        function dragMove(id, x, y) {
            let focus = focuses[id];
            if (!focus) {
                return;
            }

            var dx = x - focus.lastX;
            var dy = y - focus.lastY;
            focus.target.style.left = (parseInt(focus.target.style.left) || 0) + dx + 'px';
            focus.target.style.top = (parseInt(focus.target.style.top) || 0) + dy + 'px';
            focus.lastX = x;
            focus.lastY = y;
        }

        function dragEnd(id, x, y) {
            let focus = focuses[id];
            if (!focus) {
                return
            }
            
            focus.target.classList.remove('drag');
            delete focuses[id];
        }
        
        // Mouse
        elem.addEventListener('mousedown', function(e) {
            dragStart(e.target, null, e.clientX, e.clientY);
        }, false);

        window.addEventListener('mousemove', function(e) {
            dragMove(null, e.clientX, e.clientY);
        }, false);
        
        window.addEventListener('mouseup', function(e) {
            dragEnd(null, e.clientX, e.clientY);
        }, false);

        // Touch
        elem.addEventListener('touchstart', function(e) {
            for (var i = 0; i < e.changedTouches.length; i++) {
                var touch = e.changedTouches[i];
                dragStart(touch.target, touch.identifier, touch.clientX, touch.clientY);
            }
            e.preventDefault();
        }, false);

        elem.addEventListener('touchmove', function(e) {
            for (var i = 0; i < e.changedTouches.length; i++) {
                var touch = e.changedTouches[i];
                dragMove(touch.identifier, touch.clientX, touch.clientY);
            }
            e.preventDefault();
        }, false);
        
        elem.addEventListener('touchend', function(e) {
            for (var i = 0; i < e.changedTouches.length; i++) {
                var touch = e.changedTouches[i];
                dragEnd(touch.identifier, touch.clientX, touch.clientY);
            }
            e.preventDefault();
        }, false);
    }

    function Screen(x, y) {
        var width = 160;
        var height = 100;
        this.elem = E('div', {
            className: 'screen dragabble',
            style: {
                width: width + 'px',
                height: height + 'px',
                left: x - width / 2 + 'px',
                top: y - height / 2 + 'px',
            },
            children: [E('h3', {
                className: 'screen-name',
                textContent: 'gallifrey',
            })]
        });
    }
    
    Canvas(document.querySelector('.canvas'));
})(window, document, element.html)
