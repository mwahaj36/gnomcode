try {
      const savedTheme = localStorage.getItem('gnomcode-theme');
      if (savedTheme === 'light' || savedTheme === 'dark') document.documentElement.dataset.theme = savedTheme;
    } catch (_) {}
