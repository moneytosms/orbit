// Imported into the generated service worker: routes a notification click to the app.
self.addEventListener('notificationclick', (event) => {
  const { task_id } = event.notification.data || {}
  const action = event.action || 'open'
  event.notification.close()
  event.waitUntil(
    self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then((all) => {
      const open = all[0]
      if (open) {
        open.postMessage({ type: 'orbit-notify', action, task_id })
        return open.focus()
      }
      const q = new URLSearchParams({ notify: action })
      if (task_id) q.set('task', task_id)
      return self.clients.openWindow('/?' + q)
    }),
  )
})
