document.addEventListener('DOMContentLoaded', function() {
  // Mobile menu toggle
  const menuToggle = document.querySelector('.menu-toggle');
  const mainNav = document.querySelector('.main-nav');
  
  if (menuToggle) {
    menuToggle.addEventListener('click', function() {
      mainNav.style.display = mainNav.style.display === 'flex' ? 'none' : 'flex';
      menuToggle.classList.toggle('active');
    });
  }
  
  // Highlight active menu item
  const currentPath = window.location.pathname;
  const navLinks = document.querySelectorAll('.main-nav a:not(.github-link)');
  
  navLinks.forEach(link => {
    const href = link.getAttribute('href');
    if (currentPath.includes(href) && href !== '/') {
      link.classList.add('active');
    } else if (currentPath === '/' && href === '/') {
      link.classList.add('active');
    }
  });
  
  // We're using Jekyll's built-in Rouge highlighter instead of highlight.js
  // This avoids the unescaped HTML warnings from highlight.js
}); 