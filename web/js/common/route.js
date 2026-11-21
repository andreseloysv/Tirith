// The page's routes: #/<view>[/<project>].

/** The current route: #/<view>[/<project>]. */
export function route() {
  const [view, project] = location.hash.replace(/^#\/?/, '').split('/').map(decodeURIComponent);
  return { view: view || 'overview', project: project || '' };
}
export function go(view, project) {
  location.hash = `#/${view}${project ? `/${encodeURIComponent(project)}` : ''}`;
}
