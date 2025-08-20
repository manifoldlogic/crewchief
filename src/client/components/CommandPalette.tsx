import React, { useState } from "react";

const CommandPalette: React.FC = () => {
  const [query, setQuery] = useState("");

  // Fuzzy search implementation

  return (
    <div>
      <input value={query} onChange={(e) => setQuery(e.target.value)} />
      {/* Results list with keyboard nav */}
    </div>
  );
};

export default CommandPalette;
