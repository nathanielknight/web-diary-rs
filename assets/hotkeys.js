const hotkeys = {}

function hotkey(key, fn) {
    hotkeys[key] = fn;
}

function goto(path) {
    if (path == undefined) {
        return
    }
    let current = new URL(document.location);
    let going = new URL(path, document.location);
    if (current.href !== going.href) {
        document.location.assign(going);
    }
}

function handleHotkey(evt) {
    console.log(evt);
    if (evt.key === 'Escape') {
        // This lets you escape and access the other hotkey.
        document.activeElement.blur();
        return;
    }
    if (evt.target != document.body) {
        // If we're focusing an element, let it handle things.
        return
    }
    const handler = hotkeys[evt.key];
    if (handler != undefined) {
        console.log(`handler for ${evt.key}`)
        handler(evt);
    }
}

document.body.addEventListener('keydown', handleHotkey);

const searchInput = document.querySelector("form input[name='q']")

function focusSearch(evt) {
    evt.preventDefault()
    searchInput?.focus()
}

hotkey("n", () => goto("/new"))
hotkey("h", () => goto("/"))
hotkey("s", focusSearch)