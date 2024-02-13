import { basicSetup, EditorView } from "codemirror";
import { markdown } from "@codemirror/lang-markdown";

function setup() {
    let textarea = document.querySelector('textarea');
    if (textarea === null) {
        return;
    }

    let view = new EditorView({
        doc: textarea.value,
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
    setInterval(() => saveDraft(view), 6000);

    textarea.parentElement.onsubmit = () => {
        if (textarea != null) {
            const text = (view.state.doc as unknown) as string;
            textarea.value = text;
        }
    }
}

let savedDraft = "";

async function saveDraft(view: EditorView): Promise<void> {
    const draft = (view.state.doc as unknown) as string;
    const formData = new URLSearchParams();
    formData.append("body", draft);
    if (draft == savedDraft) {
        return
    } else {
        const result = await fetch("/draft", {
            method: "post",
            body: formData.toString(),
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
            },
        });
        if (result.ok) {
            savedDraft = draft;
        }
    }
}

setup();