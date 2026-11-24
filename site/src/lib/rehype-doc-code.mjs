// Wrap every highlighted code block in the same window chrome as <Terminal>.
// Markdown (unlike MDX) cannot swap <pre> for a component, so the chrome is added here.
const bar = (lang) => ({
  type: "element", tagName: "div", properties: { className: ["doccode__bar"], ariaHidden: "true" },
  children: [
    ...[0, 1, 2].map(() => ({ type: "element", tagName: "span", properties: {}, children: [] })),
    { type: "element", tagName: "code", properties: {}, children: [{ type: "text", value: lang }] },
  ],
});

function walk(node) {
  if (!node.children) return;
  node.children = node.children.map((child) => {
    if (child.type === "element" && child.tagName === "pre") {
      return { type: "element", tagName: "figure", properties: { className: ["doccode"] }, children: [bar(String(child.properties?.dataLanguage ?? "text")), child] };
    }
    walk(child);
    return child;
  });
}

export const rehypeDocCode = () => (tree) => walk(tree);
