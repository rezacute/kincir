// Code copy functionality for code samples
document.addEventListener('DOMContentLoaded', function() {
  console.log('DOM loaded, initializing code copy functionality');
  
  // Ensure copy buttons are added to all highlight-wrapper elements
  const highlightWrappers = document.querySelectorAll('.highlight-wrapper');
  console.log('Found highlight wrappers:', highlightWrappers.length);
  
  highlightWrappers.forEach(function(wrapper, index) {
    console.log('Processing highlight wrapper:', index);
    
    // Skip if already has a copy button
    if (wrapper.querySelector('.copy-button') || wrapper.querySelector('.manual-copy-btn')) {
      console.log('Copy button already exists for wrapper:', index);
      return;
    }
    
    // Create button element
    const copyButton = document.createElement('button');
    copyButton.className = 'copy-button manual-copy-btn';
    copyButton.setAttribute('aria-label', 'Copy code to clipboard');
    copyButton.setAttribute('onclick', 'copyCode(this)');
    copyButton.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
    
    // Add button to wrapper
    wrapper.appendChild(copyButton);
    console.log('Copy button added to wrapper:', index);
  });

  // Remove any duplicate copy buttons from highlight-wrapper elements
  highlightWrappers.forEach(function(wrapper) {
    const copyButtons = wrapper.querySelectorAll('.copy-button, .manual-copy-btn');
    if (copyButtons.length > 1) {
      // Keep only the first button
      for (let i = 1; i < copyButtons.length; i++) {
        copyButtons[i].remove();
      }
      console.log('Removed duplicate copy buttons from highlight-wrapper');
    }
    
    // ENHANCED: More thoroughly remove any "LANGUAGE-XXXX" text that appears in code blocks
    const codeElement = wrapper.querySelector('pre');
    if (codeElement) {
      // First approach: Remove if the first line contains LANGUAGE-
      if (codeElement.textContent && codeElement.textContent.trim().startsWith('LANGUAGE-')) {
        let text = codeElement.textContent;
        const lines = text.split('\n');
        
        if (lines[0].includes('LANGUAGE-')) {
          lines.shift();
          codeElement.textContent = lines.join('\n');
          console.log('Removed language indicator line from code block (method 1)');
        }
      }
      
      // Second approach: Walk the DOM and remove any text node containing LANGUAGE-
      const walkNodes = function(node) {
        // If this is a text node, check its content
        if (node.nodeType === Node.TEXT_NODE) {
          if (node.textContent.includes('LANGUAGE-')) {
            // Replace the text node with an empty text node
            node.textContent = node.textContent.replace(/LANGUAGE-[A-Za-z0-9]+/g, '');
            console.log('Removed language indicator from text node (method 2)');
          }
        } 
        // If this is an element node, recurse into children
        else if (node.nodeType === Node.ELEMENT_NODE) {
          // If we come across a specific element with LANGUAGE- class attributes, handle it
          if (node.className && node.className.includes && node.className.includes('LANGUAGE-')) {
            node.style.display = 'none';
            console.log('Hid element with LANGUAGE class (method 2)');
          }
          
          // Recurse into child nodes
          for (let i = 0; i < node.childNodes.length; i++) {
            walkNodes(node.childNodes[i]);
          }
        }
      };
      
      // Start walking the DOM from the code element
      walkNodes(codeElement);
    }
  });
});

// Function for copy button - works for both highlight-wrappers with and without buttons
function copyCode(button) {
  // Find the nearest code block
  const wrapper = button.closest('.highlight-wrapper');
  if (!wrapper) return;
  
  const codeBlock = wrapper.querySelector('.highlight');
  if (!codeBlock) return;
  
  // Get the text content from the highlight block
  const code = codeBlock.innerText;
  
  // Copy the text to the clipboard
  navigator.clipboard.writeText(code).then(function() {
    // Visual feedback
    button.classList.add('copied');
    button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>';
    
    // Reset after 2 seconds
    setTimeout(function() {
      button.classList.remove('copied');
      button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
    }, 2000);
  }).catch(function(err) {
    console.error('Could not copy text: ', err);
  });
} 