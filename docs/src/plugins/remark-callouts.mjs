import { visit } from 'unist-util-visit';

const CALLOUT_TYPES = ['NOTE', 'WARNING', 'TIP'];
const CALLOUT_RE = /^\[!(NOTE|WARNING|TIP)\]\s*/;

function serializeChildren(children) {
  return children
    .map((node) => {
      if (node.type === 'text') return escapeHtml(node.value);
      if (node.type === 'inlineCode') return `<code>${escapeHtml(node.value)}</code>`;
      if (node.type === 'strong')
        return `<strong>${serializeChildren(node.children)}</strong>`;
      if (node.type === 'emphasis')
        return `<em>${serializeChildren(node.children)}</em>`;
      if (node.type === 'link')
        return `<a href="${escapeHtml(node.url)}">${serializeChildren(node.children)}</a>`;
      if (node.type === 'html') return node.value;
      if (node.children) return serializeChildren(node.children);
      return '';
    })
    .join('');
}

function escapeHtml(str) {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function serializeBlock(node) {
  if (node.type === 'paragraph') {
    return `<p>${serializeChildren(node.children)}</p>`;
  }
  if (node.type === 'list') {
    const tag = node.ordered ? 'ol' : 'ul';
    const items = node.children
      .map((li) => `<li>${li.children.map(serializeBlock).join('')}</li>`)
      .join('\n');
    return `<${tag}>\n${items}\n</${tag}>`;
  }
  if (node.type === 'code') {
    const lang = node.lang ? ` class="language-${escapeHtml(node.lang)}"` : '';
    return `<pre><code${lang}>${escapeHtml(node.value)}</code></pre>`;
  }
  if (node.type === 'html') {
    return node.value;
  }
  if (node.children) {
    return node.children.map(serializeBlock).join('');
  }
  if (node.value) {
    return escapeHtml(node.value);
  }
  return '';
}

export default function remarkCallouts() {
  return (tree) => {
    visit(tree, 'blockquote', (node, index, parent) => {
      if (!node.children || node.children.length === 0) return;

      const firstChild = node.children[0];
      if (firstChild.type !== 'paragraph' || !firstChild.children || firstChild.children.length === 0) return;

      const firstInline = firstChild.children[0];
      if (firstInline.type !== 'text') return;

      const match = firstInline.value.match(CALLOUT_RE);
      if (!match) return;

      const calloutType = match[1];
      const typeLower = calloutType.toLowerCase();
      const typeLabel = calloutType.charAt(0) + calloutType.slice(1).toLowerCase();

      // Remove the [!TYPE] tag from the text
      const remaining = firstInline.value.slice(match[0].length);

      // Build the content paragraphs
      const contentChildren = [...node.children];

      // Update the first paragraph: remove the tag text
      if (remaining.length === 0 && firstChild.children.length === 1) {
        // The entire first paragraph was just the tag, remove it
        contentChildren.shift();
      } else {
        // Clone the first paragraph without the tag
        const newFirstChildren = [...firstChild.children];
        if (remaining.length === 0) {
          // Remove the first text node entirely
          newFirstChildren.shift();
          // If the next node starts with whitespace or newline, trim it
          if (newFirstChildren.length > 0 && newFirstChildren[0].type === 'text') {
            newFirstChildren[0] = {
              ...newFirstChildren[0],
              value: newFirstChildren[0].value.replace(/^\s*\n?/, ''),
            };
            if (newFirstChildren[0].value === '') {
              newFirstChildren.shift();
            }
          }
        } else {
          newFirstChildren[0] = { ...firstInline, value: remaining };
        }
        if (newFirstChildren.length > 0) {
          contentChildren[0] = { ...firstChild, children: newFirstChildren };
        } else {
          contentChildren.shift();
        }
      }

      // Serialize the remaining content to HTML
      const bodyHtml = contentChildren.map(serializeBlock).join('\n');

      const html = `<div class="callout callout-${typeLower}">\n  <p class="callout-label">${typeLabel}</p>\n  ${bodyHtml}\n</div>`;

      // Replace the blockquote node with an HTML node
      parent.children[index] = {
        type: 'html',
        value: html,
      };
    });
  };
}
