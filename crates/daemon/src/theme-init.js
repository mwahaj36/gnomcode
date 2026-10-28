try {
      const savedTheme = localStorage.getItem('spawnt-theme');
      if (savedTheme === 'light' || savedTheme === 'dark') document.documentElement.dataset.theme = savedTheme;
    } catch (_) {}
