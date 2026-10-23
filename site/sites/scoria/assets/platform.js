// Point the download buttons at the visitor's platform.
//
// Every address on the page is already a working direct download, and Linux is
// the one the markup ships with, because it is the platform  is developed
// on and the only one run on real hardware. This only reorders the offer:
//
//   - the hero's primary button (data-dl-main) takes the matching archive and
//     says so, and
//   - the card in a .dl-grid for the matching platform is marked [data-you],
//     which the stylesheet gives the accent border and a "Yours" chip.
//
// With JavaScript off, or on a platform none of the three matches, the page
// stands as written: three equal cards and a Linux button.
(function () {
  var files = {
    linux:   { file: '-linux-x86_64.tar.gz',   label: 'Download for Linux' },
    windows: { file: '-windows-x86_64.zip',    label: 'Download for Windows' },
    macos:   { file: '-macos-aarch64.tar.gz',  label: 'Download for macOS' },
  };

  // userAgentData where it exists, the user-agent string otherwise. Android
  // and iOS deliberately match nothing: there is no build for either, and
  // offering one of the three desktop archives would be worse than offering none.
  function detect() {
    var ua = navigator.userAgent || '';
    var plat = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || '';
    var s = (plat + ' ' + ua).toLowerCase();
    if (/android|iphone|ipad|ipod/.test(s)) return null;
    if (/win/.test(s)) return 'windows';
    if (/mac|darwin/.test(s)) return 'macos';
    if (/linux|x11|bsd/.test(s)) return 'linux';
    return null;
  }

  var me = detect();
  if (!me) return;

  var main = document.querySelector('[data-dl-main]');
  if (main && files[me]) {
    main.href = main.href.replace(/[^/]+$/, files[me].file);
    main.textContent = files[me].label;
  }

  var card = document.querySelector('.dl-grid [data-platform="' + me + '"]');
  if (card) card.setAttribute('data-you', '');
})();
