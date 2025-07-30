import React, { useState } from 'react';
import axios from 'axios';
import './App.css';

const API_BASE_URL = 'http://127.0.0.1:3000';

function App() {
  // State for text ingestion
  const [textContent, setTextContent] = useState('');
  const [textStatus, setTextStatus] = useState('');

  // State for file ingestion
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [fileStatus, setFileStatus] = useState('');

  // Handler for pasted text
  const handleTextIngest = async () => {
    if (!textContent.trim()) {
      setTextStatus('Content cannot be empty.');
      return;
    }
    setTextStatus('Ingesting text...');
    try {
      await axios.post(`${API_BASE_URL}/api/ingest/text`, {
        content: textContent,
      });
      setTextStatus('Successfully ingested text!');
      setTextContent(''); // Clear on success
    } catch (error) {
      console.error('Text ingestion error:', error);
      setTextStatus('Failed to ingest text.');
    }
  };

  // Handler for file upload
  const handleFileIngest = async () => {
    if (!selectedFile) {
      setFileStatus('Please select a file first.');
      return;
    }
    setFileStatus(`Ingesting file: ${selectedFile.name}...`);
    
    // Use FormData for file uploads
    const formData = new FormData();
    formData.append('document', selectedFile);

    try {
      await axios.post(`${API_BASE_URL}/api/ingest/file`, formData, {
        headers: {
          'Content-Type': 'multipart/form-data',
        },
      });
      setFileStatus('Successfully ingested file!');
      setSelectedFile(null); // Clear on success
    } catch (error) {
      console.error('File ingestion error:', error);
      setFileStatus('Failed to ingest file.');
    }
  };

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    if (event.target.files) {
      setSelectedFile(event.target.files[0]);
    }
  };

  return (
    <div className="App">
      <header className="App-header">
        <h1>Rust Coder AI</h1>
      </header>
      <main className="ingest-container">
        <div className="card">
          <h2>Ingest by Pasting Text</h2>
          <textarea
            value={textContent}
            onChange={(e) => setTextContent(e.target.value)}
            placeholder="Paste document content here..."
            rows={10}
          />
          <button onClick={handleTextIngest}>Ingest Text</button>
          {textStatus && <p className="status">{textStatus}</p>}
        </div>
        <div className="card">
          <h2>Ingest by Attaching File</h2>
          <input type="file" onChange={handleFileChange} />
          <button onClick={handleFileIngest} disabled={!selectedFile}>
            Ingest File
          </button>
          {fileStatus && <p className="status">{fileStatus}</p>}
        </div>
      </main>
    </div>
  );
}

export default App;