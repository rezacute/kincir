document.addEventListener('DOMContentLoaded', function() {
  // Only target pre elements within the docs-container-page class
  const preElements = document.querySelectorAll('.docs-container-page pre');
  
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
  
  preElements.forEach(function(pre) {
    // Skip if it already has a copy button
    if (pre.querySelector('.copy-button') || pre.querySelector('.manual-copy-btn')) return;
    
    // Create the copy button
    const copyButton = document.createElement('button');
    copyButton.className = 'manual-copy-btn';
    copyButton.innerHTML = copyIconSVG;
    copyButton.title = 'Copy to clipboard';
    copyButton.setAttribute('aria-label', 'Copy code to clipboard');
    
    // Add the button to the pre element
    pre.appendChild(copyButton);
    
    // Add click event to copy the code
    copyButton.addEventListener('click', function() {
      // Get the code text
      const code = pre.querySelector('code') ? pre.querySelector('code').textContent : pre.textContent;
      
      // Use the Clipboard API to copy the text
      navigator.clipboard.writeText(code).then(function() {
        console.log('Code copied to clipboard');
        
        // Visual feedback that the code was copied
        copyButton.innerHTML = checkIconSVG;
        copyButton.classList.add('copied');
        copyButton.title = 'Copied!';
        
        // Reset the button after 2 seconds
        setTimeout(function() {
          copyButton.innerHTML = copyIconSVG;
          copyButton.classList.remove('copied');
          copyButton.title = 'Copy to clipboard';
        }, 2000);
      }).catch(function(err) {
        console.error('Failed to copy: ', err);
        
        // Show error state briefly
        copyButton.style.color = '#e53e3e';
        copyButton.title = 'Copy failed';
        
        // Reset the button after 2 seconds
        setTimeout(function() {
          copyButton.innerHTML = copyIconSVG;
          copyButton.style.color = '';
          copyButton.title = 'Copy to clipboard';
        }, 2000);
      });
    });
  });
  
  // Fix broken links by adding a click handler to highlight them
  const potentialBrokenLinks = document.querySelectorAll('.docs-container-page a[href^="/"]');
  
  potentialBrokenLinks.forEach(function(link) {
    // Skip links that have .html or other extensions
    if (link.getAttribute('href').match(/\.[a-z]{2,5}$/)) return;
    
    link.addEventListener('click', function(e) {
      // Check if the link is broken by trying to follow it
      fetch(link.href)
        .then(response => {
          if (!response.ok) {
            e.preventDefault();
            console.error('Broken link detected:', link.href);
            link.style.color = '#e53e3e';
            link.style.textDecoration = 'line-through';
            link.title = 'Broken link: ' + link.href;
          }
        })
        .catch(error => {
          e.preventDefault();
          console.error('Link error:', error);
          link.style.color = '#e53e3e';
          link.style.textDecoration = 'line-through';
          link.title = 'Broken link: ' + link.href;
        });
    });
  });
});
