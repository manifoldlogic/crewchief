import * as monaco from "monaco-editor";
import React, { useRef, useEffect } from "react";

const MonacoEditor = ({ value, onChange, language }) => {
  const editorRef = useRef(null);

  useEffect(() => {
    const editor = monaco.editor.create(editorRef.current, {
      value,
      language,
      theme: "vs-dark",
    });

    editor.onDidChangeModelContent(() => onChange(editor.getValue()));

    return () => editor.dispose();
  }, []);

  return <div ref={editorRef} style={{ height: "500px" }} />;
};

export default MonacoEditor;
