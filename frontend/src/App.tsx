import React, { useState } from 'react';
import axios from 'axios';
import './App.css';

const API_BASE_URL = 'http://127.0.0.1:3000';

interface Message {
  id: number;
  sender: 'user' | 'ai';
  text: string;
  originalQuery?: string;
}

function App() {
  const [query, setQuery] = useState('');
  const [history, setHistory] = useState<Message[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [textContent, setTextContent] = useState('');
  const [textStatus, setTextStatus] = useState('');
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [fileStatus, setFileStatus] = useState('');

  const handleTextIngest = async () => {
    if (!textContent.trim()) {
      setTextStatus('Content cannot be empty.');
      return;
    }
    setTextStatus('Ingesting text...');
    try {
      await axios.post(`${API_BASE_URL}/api/ingest/text`, { content: textContent });
      setTextStatus('Successfully ingested text!');
      setTextContent('');
    } catch (error) {
      console.error('Text ingestion error:', error);
      setTextStatus('Failed to ingest text.');
    }
  };

  const handleFileIngest = async () => {
    if (!selectedFile) {
      setFileStatus('Please select a file first.');
      return;
    }
    setFileStatus(`Ingesting file: ${selectedFile.name}...`);
    const formData = new FormData();
    formData.append('document', selectedFile);
    try {
      await axios.post(`${API_BASE_URL}/api/ingest/file`, formData);
      setFileStatus('Successfully ingested file!');
      setSelectedFile(null);
      const fileInput = document.getElementById('file-input') as HTMLInputElement;
      if (fileInput) fileInput.value = '';
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

  const handleQuerySubmit = async () => {
    if (!query.trim() || isLoading) return;

    setIsLoading(true);
    const userMessage: Message = { id: Date.now(), sender: 'user', text: query };
    setHistory(prev => [...prev, userMessage]);

    try {
      const response = await axios.post(`${API_BASE_URL}/api/query`, { query });
      const aiText = response.data.response;
      const codePart = aiText.substring(aiText.indexOf('---') + 4);

      const aiMessage: Message = {
        id: Date.now() + 1,
        sender: 'ai',
        text: codePart.trim(),
        originalQuery: query,
      };
      setHistory(prev => [...prev, aiMessage]);
    } catch (error) {
      console.error('Query error:', error);
      const errorMessage: Message = {
        id: Date.now() + 1,
        sender: 'ai',
        text: 'Sorry, something went wrong while fetching the response.',
      };
      setHistory(prev => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
      setQuery('');
    }
  };

  const handleFeedback = async (originalQuery: string | undefined, code: string, upvoted: boolean) => {
    if (!originalQuery) return;
    try {
      await axios.post(`${API_BASE_URL}/api/feedback`, { query: originalQuery, code, upvoted });
      alert('Thank you for your feedback!');
    } catch (error) {
      console.error('Feedback error:', error);
      alert('Failed to send feedback.');
    }
  };

  return (
    <div className="App">
      <aside className="sidebar">
        <h2>Knowledge Ingestion</h2>
        <div className="ingest-card">
          <h3>Paste Text</h3>
          <textarea
            value={textContent}
            onChange={(e) => setTextContent(e.target.value)}
            placeholder="Paste document content..."
            rows={8}
          />
          <button onClick={handleTextIngest}>Ingest Text</button>
          {textStatus && <p className="status">{textStatus}</p>}
        </div>
        <div className="ingest-card">
          <h3>Attach File</h3>
          <input type="file" id="file-input" onChange={handleFileChange} />
          <button onClick={handleFileIngest} disabled={!selectedFile}>
            Ingest File
          </button>
          {fileStatus && <p className="status">{fileStatus}</p>}
        </div>
      </aside>

      <main className="chat-container">
        <div className="history">
          {history.map(msg => (
            <div key={msg.id} className={`message-wrapper ${msg.sender}`}>
              <div className={`message ${msg.sender}`}>
                {msg.sender === 'ai' ? (
                  <div className="ai-message">
                    {/* Replaced with a simple, reliable pre/code block */}
                    <pre>
                      <code>{msg.text}</code>
                    </pre>
                    <div className="feedback-buttons">
                      <button title="Upvote" onClick={() => handleFeedback(msg.originalQuery, msg.text, true)}>👍</button>
                      <button title="Downvote" onClick={() => handleFeedback(msg.originalQuery, msg.text, false)}>👎</button>
                    </div>
                  </div>
                ) : (
                  <p>{msg.text}</p>
                )}
              </div>
            </div>
          ))}
          {isLoading && <div className="message-wrapper ai"><div className="message ai"><p>Thinking...</p></div></div>}
        </div>
        <div className="query-input-area">
          <textarea
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyPress={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleQuerySubmit();
              }
            }}
            placeholder="Ask for Rust code..."
            disabled={isLoading}
            rows={3}
          />
          <button onClick={handleQuerySubmit} disabled={isLoading}>
            Send
          </button>
        </div>
      </main>
    </div>
  );
}

export default App;