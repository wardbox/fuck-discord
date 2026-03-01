import { marked } from 'marked';
import DOMPurify from 'dompurify';

// Configure marked for chat-style rendering
marked.setOptions({
	gfm: true,
	breaks: true
});

export function renderMarkdown(content: string): string {
	const html = marked.parse(content, { async: false }) as string;
	return DOMPurify.sanitize(html, {
		ALLOWED_TAGS: [
			'p', 'br', 'strong', 'em', 'del', 'code', 'pre',
			'a', 'ul', 'ol', 'li', 'blockquote', 'h1', 'h2',
			'h3', 'h4', 'h5', 'h6', 'hr', 'span', 'img'
		],
		ALLOWED_ATTR: ['href', 'target', 'rel', 'class', 'src', 'alt', 'loading'],
		ADD_ATTR: ['target']
	});
}
