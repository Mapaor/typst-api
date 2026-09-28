const compileBtn = document.getElementById('compileBtn');
const sourceTextarea = document.getElementById('source');
const tokenInput = document.getElementById('token');
const pdfPreview = document.getElementById('pdfPreview');
const statusDiv = document.getElementById('status');

async function compilePdf() {
    compileBtn.disabled = true;
    compileBtn.textContent = 'Compiling...';
    statusDiv.className = 'status'; // hide
    
    const sourceCode = sourceTextarea.value;
    const token = tokenInput.value.trim();

    const headers = { 'Content-Type': 'application/json' };
    if (token) {
        headers['Authorization'] = token.startsWith('Bearer ') ? token : `Bearer ${token}`;
    }

    try {
        // Using relative path to work on any domain/port
        const response = await fetch('/compile/source', {
            method: 'POST',
            headers: headers,
            body: JSON.stringify({
                source: sourceCode,
                filename: "main.typ"
            })
        });

        if (!response.ok) {
            let errorText = await response.text();
            try {
                const errObj = JSON.parse(errorText);
                if (errObj.message) errorText = errObj.message;
            } catch(e) {}
            throw new Error(`Error ${response.status}: ${errorText}`);
        }

        const pdfBlob = await response.blob();
        const blobUrl = URL.createObjectURL(pdfBlob);
        
        // Revoke old URL to prevent memory leaks if one exists
        if (pdfPreview.src && pdfPreview.src.startsWith('blob:')) {
            URL.revokeObjectURL(pdfPreview.src);
        }
        
        pdfPreview.src = blobUrl;
        statusDiv.className = 'status info';
        statusDiv.textContent = 'Compiled successfully.';
        // Auto hide success after 3 seconds
        setTimeout(() => { if(statusDiv.className === 'status info') statusDiv.className = 'status'; }, 3000);
    } catch (err) {
        statusDiv.className = 'status error';
        statusDiv.textContent = err.message;
        pdfPreview.src = '';
    } finally {
        compileBtn.disabled = false;
        compileBtn.textContent = 'Compile PDF';
    }
}

compileBtn.addEventListener('click', compilePdf);

// Allow Ctrl+Enter / Cmd+Enter to compile
sourceTextarea.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault();
        compilePdf();
    }
});