// Start URL of the installed web app (see public/manifest.webmanifest): reopens the last share
// this device viewed (the `share_last` cookie set by app/pages/share/[shareId].vue), or the home
// page if there's none yet. A server redirect rather than a page, so the app opens straight onto
// the share without flashing the home page first.
export default defineEventHandler((event) => {
  const lastShare = getCookie(event, 'share_last')

  // Room ids are Discord ids (see server/routes/host.ts's getRoomId) - anything else is a stale or
  // tampered cookie, not worth redirecting to.
  if (lastShare && /^\d+$/.test(lastShare)) {
    return sendRedirect(event, `/share/${lastShare}`)
  }

  return sendRedirect(event, '/')
})
