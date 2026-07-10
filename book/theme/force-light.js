(function() {
  try {
    localStorage.setItem('mdbook-theme', 'light');
  } catch (e) {}
  var html = document.documentElement;
  html.classList.remove('ayu', 'coal', 'navy', 'rust');
  html.classList.add('light');
})();
