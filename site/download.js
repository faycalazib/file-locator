/*
 * Download button: asks GitHub for the latest release and points the button
 * at its Windows installer (…-setup.exe), with its version and size, and the
 * "portable" link at its ZIP (…-portable.zip, lot 6.8). Without network or
 * before the first release, both lead to the releases page.
 */
(function () {
  const repo = window.PROSPECTOR_REPO;
  const releases = `https://github.com/${repo}/releases`;
  let latest = null;

  function sizeText(bytes) {
    const mb = bytes / (1024 * 1024);
    return new Intl.NumberFormat(window.i18n.lang === 'ar' ? 'ar-u-nu-latn' : window.i18n.lang, {
      maximumFractionDigits: 1,
    }).format(mb) + ' MB';
  }

  function render() {
    const button = document.getElementById('download');
    const version = document.getElementById('version');
    const all = document.getElementById('all-versions');
    const portable = document.getElementById('portable');
    all.href = releases;
    portable.href = latest?.portable ?? releases + '/latest';
    if (!latest) {
      button.href = releases + '/latest';
      version.textContent = '';
      return;
    }
    button.href = latest.url;
    version.textContent = window.i18n.t('download.version', { version: latest.version, size: sizeText(latest.size) });
  }

  window.addEventListener('langchange', render);

  document.addEventListener('DOMContentLoaded', async () => {
    render();
    if (!repo || repo.includes('OWNER')) return;
    try {
      const response = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, {
        headers: { Accept: 'application/vnd.github+json' },
      });
      if (!response.ok) return;
      const release = await response.json();
      const asset = (release.assets || []).find((a) => /-setup\.exe$/i.test(a.name));
      if (!asset) return;
      const zip = (release.assets || []).find((a) => /-portable\.zip$/i.test(a.name));
      latest = {
        url: asset.browser_download_url,
        size: asset.size,
        version: String(release.tag_name).replace(/^v/, ''),
        portable: zip ? zip.browser_download_url : null,
      };
      render();
    } catch {
      /* offline: the button keeps pointing at the releases page */
    }
  });
})();
