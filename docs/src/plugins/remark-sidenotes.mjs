import { visit, SKIP } from 'unist-util-visit';

const SIDENOTE_RE = /\[\[sn:\s*(.*?)\]\]/g;

export default function remarkSidenotes() {
  return (tree) => {
    let counter = 0;

    visit(tree, 'text', (node, index, parent) => {
      if (!parent || index === undefined) return;

      const text = node.value;
      if (!text.includes('[[sn:')) return;

      const newNodes = [];
      let lastIndex = 0;

      let match;
      // Reset the regex lastIndex since we reuse the global regex
      SIDENOTE_RE.lastIndex = 0;

      while ((match = SIDENOTE_RE.exec(text)) !== null) {
        counter++;
        const noteContent = match[1].trim();
        const beforeText = text.slice(lastIndex, match.index);

        // Add preceding text if any
        if (beforeText) {
          newNodes.push({ type: 'text', value: beforeText });
        }

        // Add the sidenote HTML
        newNodes.push({
          type: 'html',
          value:
            `<span class="sidenote-ref" aria-label="Sidenote ${counter}">${counter}</span>` +
            `<span class="sidenote"><span class="sidenote-number">${counter}. </span>${escapeHtml(noteContent)}</span>`,
        });

        lastIndex = match.index + match[0].length;
      }

      // Add any remaining text after the last match
      const trailingText = text.slice(lastIndex);
      if (trailingText) {
        newNodes.push({ type: 'text', value: trailingText });
      }

      // Replace the original text node with the new nodes
      if (newNodes.length > 0) {
        parent.children.splice(index, 1, ...newNodes);
        // Return SKIP to avoid revisiting the newly inserted nodes
        return [SKIP, index + newNodes.length];
      }
    });
  };
}

function escapeHtml(str) {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
