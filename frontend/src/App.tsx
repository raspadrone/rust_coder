import { useState } from 'react';
import axios from 'axios';
import './App.css';

const API_BASE_URL = 'http://127.0.0.1:3000';

function App() {
  const [ingestContent, setIngestContent] = useState('');
  const [ingestStatus, setIngestStatus] = useState('');

  const handleIngest = async () => {
    if (!ingestContent.trim()) {
      setIngestStatus('Content cannot be empty.');
      return;
    }
    setIngestStatus('Ingesting...');
    try {
      await axios.post(`${API_BASE_URL}/api/ingest`, {
        content: ingestContent,
      });
      setIngestStatus('Successfully ingested document!');
      setIngestContent(''); // Clear the textarea on success
    } catch (error) {
      console.error('Ingestion error:', error);
      setIngestStatus('Failed to ingest document.');
    }
  };

  return (
    <div className="App">
      <header className="App-header">
        <h1>Rust Coder AI</h1>
      </header>
      <main>
        <div className="card">
          <h2>Ingest Knowledge</h2>
          <p>Add a document to the AI's knowledge base.</p>
          <textarea
            value={ingestContent}
            onChange={(e) => setIngestContent(e.target.value)}
            placeholder="Paste document content here..."
            rows={10}
          />
          <button onClick={handleIngest}>Ingest Document</button>
          {ingestStatus && <p className="status">{ingestStatus}</p>}
        </div>
      </main>
    </div>
  );
}

export default App;