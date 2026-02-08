const ALLOWED_TAGS = new Set([
  'p',
  'br',
  'b',
  'strong',
  'i',
  'em',
  'u',
  's',
  'a',
  'ul',
  'ol',
  'li',
  'blockquote',
  'code',
  'pre',
  'h1',
  'h2',
  'h3',
  'hr',
]);

const ALLOWED_ATTRS: Record<string, Set<string>> = {
  a: new Set(['href', 'target', 'rel']),
};

function isSafeHref(href: string) {
  const h = (href || '').trim().toLowerCase();
  if (!h) return false;
  if (h.startsWith('javascript:')) return false;
  if (h.startsWith('data:')) return false;
  // allow http(s), mailto, and relative links
  return (
    h.startsWith('http://') ||
    h.startsWith('https://') ||
    h.startsWith('mailto:') ||
    h.startsWith('/') ||
    h.startsWith('#')
  );
}

export function stripHtmlToText(input: string) {
  if (typeof document === 'undefined') {
    // Server-side / non-DOM fallback (best effort).
    return String(input || '')
      .replace(/<[^>]*>/g, '')
      .trim();
  }
  const div = document.createElement('div');
  div.innerHTML = String(input || '');
  return (div.textContent || '').trim();
}

export function sanitizeHtml(input: string) {
  if (typeof document === 'undefined') return String(input || '');

  // DOMParser can behave inconsistently across webviews; avoid relying on `doc.body`.
  const root = document.createElement('div');
  root.innerHTML = String(input || '');

  const walk = (node: Node) => {
    if (node.nodeType === Node.ELEMENT_NODE) {
      const el = node as HTMLElement;
      const tag = el.tagName.toLowerCase();

      if (tag === 'script' || tag === 'style') {
        el.remove();
        return;
      }

      // Remove event handlers/styles universally
      for (const attr of Array.from(el.attributes)) {
        const name = attr.name.toLowerCase();
        if (name.startsWith('on') || name === 'style') {
          el.removeAttribute(attr.name);
        }
      }

      if (!ALLOWED_TAGS.has(tag)) {
        // Unwrap: replace element with its children (keeps inner text/content)
        const parent = el.parentNode;
        if (parent) {
          while (el.firstChild) parent.insertBefore(el.firstChild, el);
          parent.removeChild(el);
        } else {
          el.remove();
        }
        return;
      }

      // Keep only allowed attributes for this tag
      const allowedForTag = ALLOWED_ATTRS[tag] || new Set<string>();
      for (const attr of Array.from(el.attributes)) {
        const name = attr.name.toLowerCase();
        if (!allowedForTag.has(name)) {
          el.removeAttribute(attr.name);
        }
      }

      if (tag === 'a') {
        const href = el.getAttribute('href') || '';
        if (!isSafeHref(href)) {
          el.removeAttribute('href');
        }
        // Default safe link behavior
        const target = (el.getAttribute('target') || '').trim();
        if (target) el.setAttribute('target', '_blank');
        const rel = (el.getAttribute('rel') || '').trim();
        if (!rel) el.setAttribute('rel', 'nofollow noopener noreferrer');
      }
    }

    for (const child of Array.from(node.childNodes)) {
      walk(child);
    }
  };

  walk(root);
  return root.innerHTML;
}
