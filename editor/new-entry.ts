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
    setInterval(() => {
        const draft = (view.state.doc as unknown) as string;
        saveDraft(draft);
    }, 6000);

    textarea.parentElement.onsubmit = () => {
        if (textarea != null) {
            const text = (view.state.doc as unknown) as string;
            textarea.value = text;
        }
    }
}

let savedDraft = "";

async function saveDraft(draft: string): Promise<void> {
    if (draft === savedDraft) {
        return;
    }
    saveDraftLocal(draft);
    await saveDraftServer(draft);
}

const localDraftId = generateRandomString(32);

// In case auth expires while writing a draft, dump a version in localstorage.
function saveDraftLocal(draft: string): void {
    const storage = new window.Storage();
    const item = {
        timestamp: new Date().toISOString(),
        draft,
    };
    storage.setItem(localDraftId, JSON.stringify(item));
}

async function saveDraftServer(draft: string): Promise<void> {
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

function generateRandomString(length: number): string {
    let result = '';
    const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

    for (let i = 0; i < length; i++) {
        const randomIndex = Math.floor(Math.random() * characters.length);
        result += characters.charAt(randomIndex);
    }

    return result;
}