// Code copy functionality for code samples
document.addEventListener('DOMContentLoaded', function() {
  console.log('DOM loaded, initializing code copy functionality');
  
  // Copy icon SVG
  const copyIconSVG = `
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
    </svg>
  `;
  
  // Check icon SVG
  const checkIconSVG = `
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="20,6 9,17 4,12"></polyline>
    </svg>
  `;
  
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
    copyButton.className = 'manual-copy-btn';
    copyButton.setAttribute('aria-label', 'Copy code to clipboard');
    copyButton.innerHTML = copyIconSVG;
    copyButton.title = 'Copy to clipboard';
    
    // Add click event
    copyButton.addEventListener('click', function() {
      const codeElement = wrapper.querySelector('pre code') || wrapper.querySelector('pre');
      const code = codeElement ? codeElement.textContent : '';
      
      navigator.clipboard.writeText(code).then(function() {
        console.log('Code copied to clipboard');
        
        // Visual feedback
        copyButton.innerHTML = checkIconSVG;
        copyButton.classList.add('copied');
        copyButton.title = 'Copied!';
        
        // Reset after 2 seconds
        setTimeout(function() {
          copyButton.innerHTML = copyIconSVG;
          copyButton.classList.remove('copied');
          copyButton.title = 'Copy to clipboard';
        }, 2000);
      }).catch(function(err) {
        console.error('Failed to copy: ', err);
        
        // Show error state
        copyButton.style.color = '#e53e3e';
        copyButton.title = 'Copy failed';
        
        // Reset after 2 seconds
        setTimeout(function() {
          copyButton.innerHTML = copyIconSVG;
          copyButton.style.color = '';
          copyButton.title = 'Copy to clipboard';
        }, 2000);
      });
    });
    
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
    
    // Clean up any language indicators that might appear in code content
    const codeElement = wrapper.querySelector('pre');
    if (codeElement) {
      // Remove language indicators that might have been inserted into code content
      if (codeElement.textContent && codeElement.textContent.trim().startsWith('LANGUAGE-')) {
        let text = codeElement.textContent;
        const lines = text.split('\n');
        
        if (lines[0].includes('LANGUAGE-')) {
          lines.shift();
          codeElement.textContent = lines.join('\n');
        }
      }
      
      // Also clean up any stray language indicators
      const textNodes = [];
      const walker = document.createTreeWalker(
        codeElement,
        NodeFilter.SHOW_TEXT,
        null,
        false
      );
      
      let node;
      while (node = walker.nextNode()) {
        if (node.textContent.includes('LANGUAGE-')) {
          textNodes.push(node);
        }
      }
      
      textNodes.forEach(function(textNode) {
        textNode.textContent = textNode.textContent.replace(/LANGUAGE-[A-Z]+\s*/g, '');
      });
    }
  });
  
  // Also handle regular pre elements that might not be in highlight-wrapper
  const preElements = document.querySelectorAll('pre:not(.highlight-wrapper pre)');
  preElements.forEach(function(pre) {
    // Skip if already has a copy button or is inside a highlight-wrapper
    if (pre.querySelector('.copy-button') || pre.querySelector('.manual-copy-btn') || pre.closest('.highlight-wrapper')) {
      return;
    }
    
    // Create button element
    const copyButton = document.createElement('button');
    copyButton.className = 'manual-copy-btn';
    copyButton.setAttribute('aria-label', 'Copy code to clipboard');
    copyButton.innerHTML = copyIconSVG;
    copyButton.title = 'Copy to clipboard';
    
    // Add click event
    copyButton.addEventListener('click', function() {
      const code = pre.textContent;
      
      navigator.clipboard.writeText(code).then(function() {
        console.log('Code copied to clipboard');
        
        // Visual feedback
        copyButton.innerHTML = checkIconSVG;
        copyButton.classList.add('copied');
        copyButton.title = 'Copied!';
        
        // Reset after 2 seconds
        setTimeout(function() {
          copyButton.innerHTML = copyIconSVG;
          copyButton.classList.remove('copied');
          copyButton.title = 'Copy to clipboard';
        }, 2000);
      }).catch(function(err) {
        console.error('Failed to copy: ', err);
        
        // Show error state
        copyButton.style.color = '#e53e3e';
        copyButton.title = 'Copy failed';
        
        // Reset after 2 seconds
        setTimeout(function() {
          copyButton.innerHTML = copyIconSVG;
          copyButton.style.color = '';
          copyButton.title = 'Copy to clipboard';
        }, 2000);
      });
    });
    
    // Add button to pre element
    pre.style.position = 'relative';
    pre.appendChild(copyButton);
  });
});

// Legacy function for backwards compatibility
function copyCode(button) {
  const wrapper = button.closest('.highlight-wrapper') || button.closest('pre');
  const codeElement = wrapper.querySelector('code') || wrapper;
  const code = codeElement.textContent;
  
  navigator.clipboard.writeText(code).then(function() {
    console.log('Code copied to clipboard (legacy)');
    
    // Visual feedback
    const checkIconSVG = `
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="20,6 9,17 4,12"></polyline>
      </svg>
    `;
    
    const copyIconSVG = `
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
      </svg>
    `;
    
    button.innerHTML = checkIconSVG;
    button.classList.add('copied');
    button.title = 'Copied!';
    
    // Reset after 2 seconds
    setTimeout(function() {
      button.innerHTML = copyIconSVG;
      button.classList.remove('copied');
      button.title = 'Copy to clipboard';
    }, 2000);
  }).catch(function(err) {
    console.error('Failed to copy (legacy): ', err);
  });
}
