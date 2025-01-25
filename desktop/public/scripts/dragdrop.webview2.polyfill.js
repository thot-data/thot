/**
 * [See https://gist.github.com/iain-fraser/01d35885477f4e29a5a638364040d4f2]
 *
 * Polyfill for WebView2 Drag and Drop Bug
 *
 * This polyfill patches a known bug in WebView2 where drag and drop functionalities are
 * not working as expected. This script provides a mock implementation to temporarily
 * overcome the issue.
 *
 * More information:
 * - https://github.com/MicrosoftEdge/WebView2Feedback/issues/2805
 * - https://github.com/dotnet/maui/issues/2205
 *
 *
 * Developers integrating this polyfill should implement their own feature detection or
 * settings toggle to decide when to apply this fix.
 *
 * Copyright (c) 2023 Iain Fraser
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to use,
 * copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the
 * Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
 * INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
 * PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
 * HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
 * OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
 * SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
(function (opts) {
  let platform = navigator.userAgent.toLowerCase();
  if (!platform.indexOf("windows") == -1) {
    // TODO: Replace in favor of checking platform in `index.html`
    return;
  }

  let isDragging = false;
  let draggedElement = null;
  let startPosition = { x: 0, y: 0 };
  let currentOverElement = null;
  let ghostElement = null;
  let dataTransfer = null;

  /**
   * Searches for the nearest ancestor of the provided element that has the draggable attribute set to true.
   * @param {HTMLElement} element - The starting element to begin the search from.
   * @returns {HTMLElement|null} - The draggable ancestor if found, otherwise null.
   */
  function findDraggableAncestor(element) {
    while (element && element !== document.body) {
      if (element.getAttribute("draggable") === "true") {
        return element;
      }
      element = element.parentElement;
    }
    return null;
  }

  /**
   * Mock DataTransfer class mimicking the DataTransfer object available in drag & drop operations.
   */
  function MockDataTransfer() {
    this.dropEffect = "move";
    this.effectAllowed = "all";
    this.files = [];
    this.items = [];
    this.types = [];
    this.setData = function (format, data) {
      this.items.push({ format, data });
      this.types.push(format);
    };
    this.getData = function (format) {
      let item = this.items.find((i) => i.format === format);
      return item ? item.data : "";
    };
    this.clearData = function (format) {
      this.items = this.items.filter((i) => i.format !== format);
      this.types = this.types.filter((t) => t !== format);
    };
  }

  /**
   * Creates a mock drag event.
   * @param {string} type - The type of the drag event (e.g., "dragstart", "dragend", etc.)
   * @param {Object} options - Options to be passed to the event.
   * @returns {Event} - A mocked drag event.
   */
  function createMockDragEvent(type, options) {
    let event = new Event(type, options);
    if (!dataTransfer) {
      dataTransfer = new MockDataTransfer();
    }
    event.dataTransfer = dataTransfer;
    return event;
  }

  if (opts.setPointerCursor) {
    const styleTag = document.createElement("style");
    styleTag.textContent = '[draggable="true"] { cursor: pointer; }';

    // Insert the style tag as the first child of the head element
    const head = document.head;
    if (head.children.length > 0) {
      head.insertBefore(styleTag, head.children[0]);
    } else {
      head.appendChild(styleTag);
    }
  }

  document.addEventListener("mousedown", function (e) {
    // Only proceed if the left mouse button is clicked
    if (e.button !== 0) return;

    let draggableElement = findDraggableAncestor(e.target);

    if (!draggableElement) return;

    // Override the existing functionality
    draggableElement.setAttribute("draggable", "false");

    // Update state for dragging
    isDragging = true;
    draggedElement = draggableElement;
    startPosition.x = e.clientX;
    startPosition.y = e.clientY;

    // Fire dragstart event
    draggedElement.dispatchEvent(
      createMockDragEvent("dragstart", {
        bubbles: true,
        cancelable: true,
        clientX: e.clientX,
        clientY: e.clientY,
      })
    );
  });

  document.addEventListener("mousemove", function (e) {
    if (!isDragging) return;

    if (!ghostElement) {
      // Create a "ghost" clone for visual dragging
      ghostElement = draggedElement.cloneNode(true);
      applyStylesToGhost(draggedElement, ghostElement);
    }

    // Update ghost position
    let x = e.clientX - startPosition.x;
    let y = e.clientY - startPosition.y;
    ghostElement.style.transform = `translate(${x}px, ${y}px)`;

    // Check for drag over target
    ghostElement.style.display = "none";
    let elementBelow = document.elementFromPoint(e.clientX, e.clientY);
    ghostElement.style.display = "";

    if (elementBelow) {
      elementBelow.dispatchEvent(
        createMockDragEvent("dragover", {
          bubbles: true,
          cancelable: true,
          clientX: e.clientX,
          clientY: e.clientY,
        })
      );

      if (elementBelow !== currentOverElement) {
        if (currentOverElement) {
          currentOverElement.dispatchEvent(
            createMockDragEvent("dragleave", {
              bubbles: true,
              cancelable: true,
              clientX: e.clientX,
              clientY: e.clientY,
            })
          );
        }

        elementBelow.dispatchEvent(
          createMockDragEvent("dragenter", {
            bubbles: true,
            cancelable: true,
            clientX: e.clientX,
            clientY: e.clientY,
          })
        );
        currentOverElement = elementBelow;
      }
    }
  });

  document.addEventListener("mouseup", function (e) {
    // Only drop and dragend if dragging the event was triggerd with button 0 (left-click)
    if (!isDragging || e.button !== 0) return;

    // Fire drop event if we have a target
    if (currentOverElement) {
      currentOverElement.dispatchEvent(
        createMockDragEvent("drop", {
          bubbles: true,
          cancelable: true,
        })
      );
    }

    // Fire dragend event
    draggedElement.dispatchEvent(
      createMockDragEvent("dragend", {
        bubbles: true,
        cancelable: true,
      })
    );

    // Cleanup
    isDragging = false;
    draggedElement.setAttribute("draggable", "true");
    if (ghostElement) {
      document.body.removeChild(ghostElement);
    }
    ghostElement = null;
    draggedElement = null;
    currentOverElement = null;
    dataTransfer = null;
  });

  /**
   * Applies computed styles from the original element to the ghost element.
   * @param {HTMLElement} original - The original draggable element.
   * @param {HTMLElement} ghost - The cloned "ghost" element.
   */
  function applyStylesToGhost(original, ghost) {
    let computedStyles = window.getComputedStyle(original);
    for (let prop of computedStyles) {
      ghost.style[prop] = computedStyles[prop];
    }

    // Positioning and z-index
    let rect = original.getBoundingClientRect();
    ghost.style.position = "fixed";
    ghost.style.left = `${rect.left}px`;
    ghost.style.top = `${rect.top}px`;
    ghost.style.zIndex = "1000";
    ghost.style.opacity = "0.7";

    document.body.appendChild(ghost);
  }
})({ setPointerCursor: true });
