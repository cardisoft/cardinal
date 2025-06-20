import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { once } from '@tauri-apps/api/event';
import { List, AutoSizer } from 'react-virtualized';
import 'react-virtualized/styles.css';
import "./App.css";

function App() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState([]);
  const [isInitialized, setIsInitialized] = useState(false);

  once('init_completed', () => {
    setIsInitialized(true);
  });

  const handleSearch = async () => {
    const searchResults = await invoke("search", { query });
    setResults(searchResults);
  };

  const rowRenderer = ({ key, index, style }) => {
    return (
      <div key={key} style={style} className="row">
        {results[index]}
      </div>
    );
  };

  return (
    <main className="container">
      <div className="search-container">
        <input
          id="search-input"
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search for files and folders..."
          onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
          spellCheck={false}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
        />
        <button onClick={handleSearch}>Search</button>
      </div>
      <div className="results-container" style={{ flex: 1 }}>
        <AutoSizer>
          {({ height, width }) => (
            <List
              width={width}
              height={height}
              rowCount={results.length}
              rowHeight={30} // Adjust row height as needed
              rowRenderer={rowRenderer}
            />
          )}
        </AutoSizer>
      </div>
      <div className="status-bar">
        {isInitialized ? 'Initialized' : 'Initializing...'}
      </div>
    </main>
  );
}

export default App;
