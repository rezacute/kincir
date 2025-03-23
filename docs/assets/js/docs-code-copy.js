document.addEventListener('DOMContentLoaded', function() {
  // Only target pre elements within the docs-container-page class
  const preElements = document.querySelectorAll('.docs-container-page pre');
  
  preElements.forEach(function(pre) {
    // Skip if it already has a copy button
    if (pre.querySelector('.copy-button')) return;
    
    // Create the copy button
    const copyButton = document.createElement('button');
    copyButton.className = 'copy-button';
    copyButton.textContent = 'Copy';
    copyButton.title = 'Copy to clipboard';
    
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
        copyButton.textContent = 'Copied!';
        copyButton.classList.add('copied');
        
        // Reset the button after 2 seconds
        setTimeout(function() {
          copyButton.textContent = 'Copy';
          copyButton.classList.remove('copied');
        }, 2000);
      }).catch(function(err) {
        console.error('Failed to copy: ', err);
        copyButton.textContent = 'Error!';
        
        // Reset the button after 2 seconds
        setTimeout(function() {
          copyButton.textContent = 'Copy';
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