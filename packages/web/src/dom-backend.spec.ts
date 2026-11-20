import { describe, it, expect } from 'vitest';
import { Window } from 'happy-dom';
import { DomBackend, styleKeyToCss, type PlatformOp } from './dom-backend';

function setup() {
  const win = new Window({ url: 'http://localhost/' });
  const doc = win.document;
  doc.body.innerHTML = '<div id="host"></div>';
  const host = doc.querySelector<HTMLElement>('#host')!;
  const backend = new DomBackend(doc);
  return { win, doc, host, backend };
}

describe('DomBackend — PlatformOp application', () => {
  it('creates, appends, inserts at index, sets text/props/styles and removes', () => {
    const { host, backend } = setup();
    const ops: PlatformOp[] = [
      { kind: 'Create', node_id: 1, view_type: 'View' },
      { kind: 'Create', node_id: 2, view_type: 'Text' },
      { kind: 'Append', parent_id: 1, child_id: 2 },
      { kind: 'SetText', node_id: 2, text: 'second' },
      {
        kind: 'SetProps',
        node_id: 1,
        props: {
          styles: { backgroundColor: '#ff0000', padding: '8px' },
          properties: { 'attr:id': 'root', class: 'box' },
        },
      },
      { kind: 'Create', node_id: 3, view_type: 'View' },
      { kind: 'Insert', parent_id: 1, child_id: 3, index: 0 },
      { kind: 'SetText', node_id: 3, text: 'first' },
    ];

    backend.apply(ops);

    const root = backend.getNode(1) as HTMLElement;
    expect(root.tagName).toBe('NA-VIEW');
    expect(host.contains(root)).toBe(false); // ops build a detached tree
    expect(root.getAttribute('id')).toBe('root');
    expect(root.getAttribute('class')).toBe('box');
    expect(root.style.padding).toBe('8px');
    expect(root.style.getPropertyValue('background-color')).toBeTruthy();
    expect(root.childNodes[0]).toBe(backend.getNode(3));
    expect(root.childNodes[1]).toBe(backend.getNode(2));
    expect(root.textContent).toBe('firstsecond');

    backend.apply([{ kind: 'Remove', parent_id: 1, child_id: 2 }]);
    expect(root.childNodes.length).toBe(1);

    backend.apply([{ kind: 'Destroy', node_id: 3 }]);
    expect(root.childNodes.length).toBe(0);
    expect(backend.hasNode(3)).toBe(false);
  });

  it('supports text/comment view types, SetFrame and SetProps.text', () => {
    const { backend } = setup();
    backend.apply([
      { kind: 'Create', node_id: 1, view_type: 'View' },
      { kind: 'Create', node_id: 2, view_type: '#text' },
      { kind: 'Create', node_id: 3, view_type: '#comment' },
      { kind: 'Append', parent_id: 1, child_id: 2 },
      { kind: 'Append', parent_id: 1, child_id: 3 },
      { kind: 'SetText', node_id: 2, text: 'text content' },
      { kind: 'SetProps', node_id: 1, props: { text: 'replaced' } },
      { kind: 'SetFrame', node_id: 1, x: 10, y: 20, width: 100, height: 50 },
    ]);

    const root = backend.getNode(1) as HTMLElement;
    expect(backend.getNode(2)!.nodeType).toBe(3);
    expect(backend.getNode(2)!.nodeValue).toBe('text content');
    expect(backend.getNode(3)!.nodeType).toBe(8);
    expect(root.textContent).toBe('replaced');
    expect(root.style.position).toBe('absolute');
    expect(root.style.left).toBe('10px');
    expect(root.style.top).toBe('20px');
    expect(root.style.width).toBe('100px');
    expect(root.style.height).toBe('50px');
  });

  it('reuses pre-registered nodes and wires DOM listeners', () => {
    const { win, doc, backend } = setup();
    const pre = doc.createElement('div');
    doc.body.appendChild(pre);
    backend.registerNode(7, pre);
    backend.apply([{ kind: 'Create', node_id: 7, view_type: 'View' }]);
    expect(backend.getNode(7)).toBe(pre);

    let clicks = 0;
    backend.addEventListener(7, 'click', () => {
      clicks++;
    });
    pre.dispatchEvent(new win.Event('click'));
    expect(clicks).toBe(1);

    backend.removeListenersFor(7);
    pre.dispatchEvent(new win.Event('click'));
    expect(clicks).toBe(1);
  });

  it('converts camelCase style keys to CSS names', () => {
    expect(styleKeyToCss('backgroundColor')).toBe('background-color');
    expect(styleKeyToCss('borderTopWidth')).toBe('border-top-width');
    expect(styleKeyToCss('padding')).toBe('padding');
  });
});
