import { Editor, rootCtx } from "@milkdown/core";
import { commonmark } from "@milkdown/preset-commonmark";
import { history } from "@milkdown/plugin-history";
import { listener, listenerCtx } from "@milkdown/plugin-listener";
import { nord } from "@milkdown/theme-nord"

import '@milkdown/theme-nord/style.css';

let content: string = '';

export function editorContents(): string {
    return content;
}

function changeHandler(_ctx: unknown, markdown: string, _prevMarkdown: string | null): void {
    content = markdown;
}

const editor = await Editor
    .make()
    .config(ctx => {
        ctx.set(rootCtx, "#editor");
        ctx.get(listenerCtx).markdownUpdated(changeHandler)
    })
    .config(nord)
    .use(listener)
    .use(commonmark)
    .use(history)
    .create();
