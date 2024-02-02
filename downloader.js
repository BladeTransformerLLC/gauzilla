async function download_splat(url_param) {
    const url = new URL(
        url_param,
        "https://huggingface.co/datasets/satyoshi/gauzilla-data/resolve/main/",
    );

    const req = await fetch(url, {
        mode: "cors",
        credentials: "omit",
    });
    console.log(req);
    if (req.status != 200)
        throw new Error("download_splat(): HTTP status: " + req.status + ", failed to load " + req.url);

    const reader = req.body.getReader();
    let splatData = new Uint8Array(req.headers.get("content-length"));
    let bytesRead = 0;
    while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        splatData.set(value, bytesRead);
        bytesRead += value.length;

        /*
        FIXME: fails with large splat files:

        Uncaught (in promise) DOMException:
        Failed to execute 'postMessage' on 'DedicatedWorkerGlobalScope':
        Data cannot be cloned, out of memory
        */
        postMessage({
            bytes: bytesRead,
            buffer: splatData,
        });
    }
}


/*
async function download_splat2() {
    const url = new URL(
        "train.splat",
        "https://huggingface.co/cakewalk/splat-data/resolve/main/",
    );

    const req = await fetch(url, {
        mode: "cors",
        credentials: "omit",
    });
    console.log(req);
    if (req.status != 200)
        throw new Error("download_splat(): HTTP status: " + req.status + ", failed to load " + req.url);

    const reader = req.body.getReader();
    const cl = parseInt(req.headers.get("content-length"));
    let bytesRead = 0;
    while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        bytesRead += value.length;

        postMessage({
            conlen: cl,
            bytes: bytesRead,
            chunk: value,
        });
    }
}

(async () => {
    await download_splat2();
})();
*/


self.onmessage = async function(event) {
    console.log("downloader.js: Received message from Rust:", event.data);
    download_splat(event.data);
};
