import { basicSetup, EditorView } from "codemirror";
import { markdown } from "@codemirror/lang-markdown";

const docHashKey = "web-diary-rs-draft-doc-hash"
const docKey = "web-diary-rs-draft-doc";
const storage = window.localStorage;

function setup() {
    let textarea = document.querySelector('textarea');
    if (textarea == null) {
        return;
    }

    let view = new EditorView({
        doc: storage.getItem(docKey) ?? '',
        extensions: [
            EditorView.lineWrapping,
            basicSetup,
            markdown({}),
        ],
    })
    textarea.insertAdjacentElement("afterend", view.dom);
    textarea.style.display = "none";
    if (textarea.parentElement == null) {
        return;
    }
    textarea.parentElement.onsubmit = () => {
        if (textarea != null) {
            const text = (view.state.doc as unknown) as string;
            textarea.value = text;
            setTimeout(() => storeTextHash(text), 1);
        }
    }
}

async function storeTextHash(text: string): Promise<void> {
    const textBytes = (new TextEncoder()).encode(text);
    const digest = await crypto.subtle.digest("SHA-256", textBytes);
    const digestBytes = new Uint8Array(digest);
    const digestString = Array.from(digestBytes).map((b) => {
        let s = b.toString(16);
        return b < 0x10 ? '0' + s : s;
    }).join('');
    storage.setItem(docHashKey, digestString);
    storage.setItem(docKey, text);
}

setup();