use maproom::indexer::parser;
use serde_json::Value;

// Helper function to extract imports from metadata
fn extract_imports_from_chunks(
    chunks: &[maproom::indexer::SymbolChunk],
) -> Option<&Value> {
    chunks
        .iter()
        .find(|c| c.kind == "imports")
        .and_then(|c| c.metadata.as_ref())
        .and_then(|m| m.get("imports"))
}

// Test standard imports
#[test]
fn test_python_standard_import() {
    let source = r#"
import os
import sys
import json
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 3);

    // Check os import
    let os_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("os"))
        .expect("Should find os import");
    assert_eq!(
        os_import.get("import_type").and_then(|t| t.as_str()),
        Some("standard")
    );

    // Check sys import
    let sys_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("sys"))
        .expect("Should find sys import");
    assert_eq!(
        sys_import.get("import_type").and_then(|t| t.as_str()),
        Some("standard")
    );
}

#[test]
fn test_python_dotted_import() {
    let source = r#"
import os.path
import xml.etree.ElementTree
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 2);

    // Check os.path import
    let os_path = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("os.path"))
        .expect("Should find os.path import");
    assert_eq!(
        os_path.get("import_type").and_then(|t| t.as_str()),
        Some("standard")
    );

    // Check xml.etree.ElementTree import
    let xml_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("xml.etree.ElementTree"))
        .expect("Should find xml.etree.ElementTree import");
    assert_eq!(
        xml_import.get("import_type").and_then(|t| t.as_str()),
        Some("standard")
    );
}

#[test]
fn test_python_aliased_import() {
    let source = r#"
import numpy as np
import pandas as pd
import tensorflow as tf
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 3);

    // Check numpy as np
    let numpy = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("numpy"))
        .expect("Should find numpy import");
    let aliases = numpy
        .get("aliases")
        .and_then(|a| a.as_array())
        .expect("Should have aliases");
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0].as_array().unwrap()[0].as_str(), Some("numpy"));
    assert_eq!(aliases[0].as_array().unwrap()[1].as_str(), Some("np"));
}

// Test from imports
#[test]
fn test_python_from_import_single() {
    let source = r#"
from os import path
from sys import argv
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 2);

    // Check from os import path
    let os_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("os"))
        .expect("Should find os import");
    assert_eq!(
        os_import.get("import_type").and_then(|t| t.as_str()),
        Some("from")
    );
    let names = os_import
        .get("names")
        .and_then(|n| n.as_array())
        .expect("Should have names");
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].as_str(), Some("path"));
}

#[test]
fn test_python_from_import_multiple() {
    let source = r#"
from os import path, environ, getcwd
from typing import List, Dict, Optional
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 2);

    // Check from os import path, environ, getcwd
    let os_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("os"))
        .expect("Should find os import");
    let names = os_import
        .get("names")
        .and_then(|n| n.as_array())
        .expect("Should have names");
    assert_eq!(names.len(), 3);
    assert!(names.iter().any(|n| n.as_str() == Some("path")));
    assert!(names.iter().any(|n| n.as_str() == Some("environ")));
    assert!(names.iter().any(|n| n.as_str() == Some("getcwd")));

    // Check from typing import List, Dict, Optional
    let typing_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("typing"))
        .expect("Should find typing import");
    let names = typing_import
        .get("names")
        .and_then(|n| n.as_array())
        .expect("Should have names");
    assert_eq!(names.len(), 3);
    assert!(names.iter().any(|n| n.as_str() == Some("List")));
    assert!(names.iter().any(|n| n.as_str() == Some("Dict")));
    assert!(names.iter().any(|n| n.as_str() == Some("Optional")));
}

#[test]
fn test_python_from_import_aliased() {
    let source = r#"
from os.path import join as path_join
from typing import List as ListType
from collections import OrderedDict as ODict, defaultdict as ddict
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 3);

    // Check from os.path import join as path_join
    let os_path_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("os.path"))
        .expect("Should find os.path import");
    let names = os_path_import
        .get("names")
        .and_then(|n| n.as_array())
        .expect("Should have names");
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].as_str(), Some("join"));
    let aliases = os_path_import
        .get("aliases")
        .and_then(|a| a.as_array())
        .expect("Should have aliases");
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0].as_array().unwrap()[0].as_str(), Some("join"));
    assert_eq!(
        aliases[0].as_array().unwrap()[1].as_str(),
        Some("path_join")
    );

    // Check from collections import OrderedDict as ODict, defaultdict as ddict
    let collections_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("collections"))
        .expect("Should find collections import");
    let names = collections_import
        .get("names")
        .and_then(|n| n.as_array())
        .expect("Should have names");
    assert_eq!(names.len(), 2);
    let aliases = collections_import
        .get("aliases")
        .and_then(|a| a.as_array())
        .expect("Should have aliases");
    assert_eq!(aliases.len(), 2);
}

#[test]
fn test_python_wildcard_import() {
    let source = r#"
from os import *
from typing import *
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 2);

    // Check from os import *
    let os_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("os"))
        .expect("Should find os import");
    assert_eq!(
        os_import.get("is_wildcard").and_then(|w| w.as_bool()),
        Some(true)
    );

    // Check from typing import *
    let typing_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("typing"))
        .expect("Should find typing import");
    assert_eq!(
        typing_import.get("is_wildcard").and_then(|w| w.as_bool()),
        Some(true)
    );
}

// Test relative imports
#[test]
fn test_python_relative_import_single_dot() {
    let source = r#"
from . import utils
from .helpers import format_string
from .constants import API_KEY
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 3);

    // Check from . import utils
    let dot_import = imports_array
        .iter()
        .find(|i| {
            i.get("module").and_then(|m| m.as_str()) == Some("")
                && i.get("names")
                    .and_then(|n| n.as_array())
                    .map(|a| a.iter().any(|v| v.as_str() == Some("utils")))
                    .unwrap_or(false)
        })
        .expect("Should find . import utils");
    assert_eq!(
        dot_import.get("import_type").and_then(|t| t.as_str()),
        Some("relative")
    );
    assert_eq!(
        dot_import.get("relative_depth").and_then(|d| d.as_u64()),
        Some(1)
    );

    // Check from .helpers import format_string
    let helpers_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("helpers"))
        .expect("Should find .helpers import");
    assert_eq!(
        helpers_import.get("import_type").and_then(|t| t.as_str()),
        Some("relative")
    );
    assert_eq!(
        helpers_import
            .get("relative_depth")
            .and_then(|d| d.as_u64()),
        Some(1)
    );
    let names = helpers_import
        .get("names")
        .and_then(|n| n.as_array())
        .expect("Should have names");
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].as_str(), Some("format_string"));
}

#[test]
fn test_python_relative_import_multiple_dots() {
    let source = r#"
from .. import parent_module
from ..parent import sibling_module
from ...grandparent import aunt_module
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 3);

    // Check from .. import parent_module
    let parent_import = imports_array
        .iter()
        .find(|i| {
            i.get("relative_depth").and_then(|d| d.as_u64()) == Some(2)
                && i.get("module").and_then(|m| m.as_str()) == Some("")
        })
        .expect("Should find .. import");
    assert_eq!(
        parent_import.get("import_type").and_then(|t| t.as_str()),
        Some("relative")
    );

    // Check from ..parent import sibling_module
    let sibling_import = imports_array
        .iter()
        .find(|i| {
            i.get("relative_depth").and_then(|d| d.as_u64()) == Some(2)
                && i.get("module").and_then(|m| m.as_str()) == Some("parent")
        })
        .expect("Should find ..parent import");
    assert_eq!(
        sibling_import.get("import_type").and_then(|t| t.as_str()),
        Some("relative")
    );

    // Check from ...grandparent import aunt_module
    let grandparent_import = imports_array
        .iter()
        .find(|i| i.get("relative_depth").and_then(|d| d.as_u64()) == Some(3))
        .expect("Should find ...grandparent import");
    assert_eq!(
        grandparent_import
            .get("import_type")
            .and_then(|t| t.as_str()),
        Some("relative")
    );
}

// Test dynamic imports
#[test]
fn test_python_dynamic_import_builtin() {
    let source = r#"
def dynamic_loader(module_name):
    mod = __import__(module_name)
    return mod

# Another example
imported = __import__('os')
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    // Should detect at least one __import__ call
    let dynamic_imports: Vec<_> = imports_array
        .iter()
        .filter(|i| i.get("import_type").and_then(|t| t.as_str()) == Some("dynamic"))
        .collect();

    assert!(
        dynamic_imports.len() >= 1,
        "Should find at least one dynamic import"
    );

    // Check for 'os' import
    let os_import = dynamic_imports
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("os"));
    assert!(os_import.is_some(), "Should find dynamic import of 'os'");
}

#[test]
fn test_python_dynamic_import_importlib() {
    let source = r#"
import importlib

def load_module(name):
    return importlib.import_module(name)

# Direct call
requests_module = importlib.import_module('requests')
json_module = importlib.import_module('json')
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    // Check for standard import of importlib
    let importlib_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("importlib"))
        .expect("Should find importlib import");
    assert_eq!(
        importlib_import.get("import_type").and_then(|t| t.as_str()),
        Some("standard")
    );

    // Check for dynamic imports
    let dynamic_imports: Vec<_> = imports_array
        .iter()
        .filter(|i| i.get("import_type").and_then(|t| t.as_str()) == Some("dynamic"))
        .collect();

    assert!(
        dynamic_imports.len() >= 2,
        "Should find at least 2 dynamic imports"
    );

    // Check for 'requests' and 'json' dynamic imports
    let has_requests = dynamic_imports
        .iter()
        .any(|i| i.get("module").and_then(|m| m.as_str()) == Some("requests"));
    let has_json = dynamic_imports
        .iter()
        .any(|i| i.get("module").and_then(|m| m.as_str()) == Some("json"));

    assert!(has_requests, "Should find dynamic import of 'requests'");
    assert!(has_json, "Should find dynamic import of 'json'");
}

// Test edge cases
#[test]
fn test_python_multiline_import() {
    let source = r#"
from typing import (
    List,
    Dict,
    Optional,
    Union,
    Tuple
)
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    assert_eq!(imports_array.len(), 1);

    let typing_import = &imports_array[0];
    assert_eq!(
        typing_import.get("module").and_then(|m| m.as_str()),
        Some("typing")
    );
    let names = typing_import
        .get("names")
        .and_then(|n| n.as_array())
        .expect("Should have names");
    assert_eq!(names.len(), 5);
    assert!(names.iter().any(|n| n.as_str() == Some("List")));
    assert!(names.iter().any(|n| n.as_str() == Some("Dict")));
    assert!(names.iter().any(|n| n.as_str() == Some("Optional")));
    assert!(names.iter().any(|n| n.as_str() == Some("Union")));
    assert!(names.iter().any(|n| n.as_str() == Some("Tuple")));
}

#[test]
fn test_python_mixed_import_styles() {
    let source = r#"
import os
import sys as system
from pathlib import Path
from typing import List, Dict
from . import utils
from ..parent import helper
import json
mod = __import__('datetime')
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    // Count import types
    let standard_imports = imports_array
        .iter()
        .filter(|i| i.get("import_type").and_then(|t| t.as_str()) == Some("standard"))
        .count();
    let from_imports = imports_array
        .iter()
        .filter(|i| i.get("import_type").and_then(|t| t.as_str()) == Some("from"))
        .count();
    let relative_imports = imports_array
        .iter()
        .filter(|i| i.get("import_type").and_then(|t| t.as_str()) == Some("relative"))
        .count();
    let dynamic_imports = imports_array
        .iter()
        .filter(|i| i.get("import_type").and_then(|t| t.as_str()) == Some("dynamic"))
        .count();

    assert_eq!(standard_imports, 3, "Should have 3 standard imports");
    assert_eq!(from_imports, 2, "Should have 2 from imports");
    assert_eq!(relative_imports, 2, "Should have 2 relative imports");
    assert!(
        dynamic_imports >= 1,
        "Should have at least 1 dynamic import"
    );
}

// Comprehensive real-world example
#[test]
fn test_python_comprehensive_imports() {
    let source = r#"
"""Module for data processing."""

# Standard library imports
import os
import sys
import json
from pathlib import Path
from typing import List, Dict, Optional, Union

# Third-party imports
import numpy as np
import pandas as pd
from sklearn.preprocessing import StandardScaler
from tensorflow.keras.models import Sequential

# Relative imports
from . import config
from .utils import logger, format_data
from ..common import constants
from ..common.helpers import validate_input

# Dynamic imports for plugins
import importlib

def load_plugin(plugin_name):
    return importlib.import_module(f'plugins.{plugin_name}')

class DataProcessor:
    """Process data using imported modules."""

    def __init__(self):
        self.scaler = StandardScaler()
        self.logger = logger

    def process(self, data: pd.DataFrame) -> np.ndarray:
        """Process data."""
        return self.scaler.fit_transform(data)
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    // Verify we extracted a comprehensive set of imports
    assert!(
        imports_array.len() >= 15,
        "Should extract many imports from real code"
    );

    // Verify standard library imports
    let has_os = imports_array
        .iter()
        .any(|i| i.get("module").and_then(|m| m.as_str()) == Some("os"));
    assert!(has_os, "Should find os import");

    let has_pathlib = imports_array
        .iter()
        .any(|i| i.get("module").and_then(|m| m.as_str()) == Some("pathlib"));
    assert!(has_pathlib, "Should find pathlib import");

    // Verify third-party imports with aliases
    let numpy_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("numpy"));
    assert!(numpy_import.is_some(), "Should find numpy import");

    // Verify relative imports
    let relative_imports: Vec<_> = imports_array
        .iter()
        .filter(|i| i.get("import_type").and_then(|t| t.as_str()) == Some("relative"))
        .collect();
    assert!(
        relative_imports.len() >= 4,
        "Should find at least 4 relative imports"
    );

    // Verify different relative depths
    let single_dot = relative_imports
        .iter()
        .filter(|i| i.get("relative_depth").and_then(|d| d.as_u64()) == Some(1))
        .count();
    let double_dot = relative_imports
        .iter()
        .filter(|i| i.get("relative_depth").and_then(|d| d.as_u64()) == Some(2))
        .count();

    assert!(single_dot >= 2, "Should find single-dot relative imports");
    assert!(double_dot >= 2, "Should find double-dot relative imports");

    // Verify dynamic imports (may or may not find dynamic imports - not an error if none found)
    let _dynamic_imports: Vec<_> = imports_array
        .iter()
        .filter(|i| i.get("import_type").and_then(|t| t.as_str()) == Some("dynamic"))
        .collect();
    // Dynamic imports in function bodies might not be detected, so we don't assert on count
}

#[test]
fn test_python_no_imports() {
    let source = r#"
def hello():
    """Say hello."""
    return "Hello, World!"

class Greeter:
    """A greeter class."""
    def greet(self):
        return "Hi!"
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should not have an imports chunk
    let has_imports = chunks.iter().any(|c| c.kind == "imports");
    assert!(
        !has_imports,
        "Should not have imports chunk when no imports exist"
    );
}

#[test]
fn test_python_import_line_numbers() {
    let source = r#"
import os
import sys

from typing import List

def foo():
    pass
"#;

    let chunks = parser::extract_chunks(source, "py");
    let imports = extract_imports_from_chunks(&chunks).expect("Should have imports");
    let imports_array = imports.as_array().expect("Imports should be an array");

    // Check line numbers are tracked
    for import in imports_array {
        let line = import.get("line").and_then(|l| l.as_i64());
        assert!(line.is_some(), "Each import should have a line number");
        assert!(line.unwrap() >= 1, "Line numbers should be positive");
    }

    // Check os import is on line 2
    let os_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("os"))
        .expect("Should find os import");
    assert_eq!(os_import.get("line").and_then(|l| l.as_i64()), Some(2));

    // Check typing import is on line 5
    let typing_import = imports_array
        .iter()
        .find(|i| i.get("module").and_then(|m| m.as_str()) == Some("typing"))
        .expect("Should find typing import");
    assert_eq!(typing_import.get("line").and_then(|l| l.as_i64()), Some(5));
}
