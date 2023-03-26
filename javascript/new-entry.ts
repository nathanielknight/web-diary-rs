import { basicSetup, EditorView } from "codemirror";
import { markdown } from "@codemirror/lang-markdown";

function setup() {
    let textarea = document.querySelector('textarea');
    if (textarea == null) {
        return;
    }

    let view = new EditorView({
        doc: '',
        extensions: [
            EditorView.lineWrapping,
            basicSetup,
            markdown({}),
        ]
    })
    textarea.insertAdjacentElement("afterend", view.dom);
    textarea.style.display = "none";
    if (textarea.parentElement == null) {
        return;
    }
    textarea.parentElement.onsubmit = () => {
        textarea.value = (view.state.doc as unknown) as string;
    }
}
setup();