'use strict';

var element = (function(window, document) {
  var element = {};

  element.html = function(tagName, params) {
    var elem = document.createElement(tagName);
    loadParams(elem, params);
    return elem;
  };

  element.svg = function(tagName, params) {
    var elem = document.createElementNS('http://www.w3.org/2000/svg', tagName);
    loadParams(elem, params);
    return elem;
  };

  function loadParams(elem, params) {
    if (!params) return;

    if (params.parent) {
      params.parent.appendChild(elem);
      delete params.parent;
    }

    if (params.children) {
      var fragment = document.createDocumentFragment();
      params.children.forEach(function(child) {
        if (!(child instanceof Node)) {
          child = document.createTextNode(child);
        }
        fragment.appendChild(child);
      });

      elem.appendChild(fragment);
      delete params.children;
    }

    if (params.className) {
      var className;
      if (params.className instanceof Array) {
        var classList = [];
        params.className.forEach(function(className) {
          if (className instanceof Array) {
            classList.concat(className);
          } else {
            classList.push(className);
          }
        });

        className = classList.join(' ').trim();
      } else {
        className = params.className;
      }

      elem.setAttribute('class', className);
      delete params.className;
    }

    if (params.attributes) {
      for (var attribute in params.attributes) {
        if (params.attributes[attribute] !== undefined) {
          elem.setAttribute(attribute, params.attributes[attribute]);
        }
      }
      delete params.attributes;
    }

    if (params.dataset) {
      for (var attribute in params.dataset) {
        if (params.dataset[attribute] !== undefined) {
          elem.dataset[attribute] = params.dataset[attribute];
        }
      }
      delete params.dataset;
    }

    if (params.style) {
      for (var key in params.style) {
        var val = params.style[key];
        if (val !== undefined) {
          elem.style[key] = val;
        }
      }
      delete params.style;
    }

    for (var key in params) {
      var val = params[key];
      if (val !== undefined) {
        elem[key] = val;
      }
    }
  }

  return element;
})(window, document);
