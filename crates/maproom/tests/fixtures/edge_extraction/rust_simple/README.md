# rust_simple fixture

Ground-truth same-file `calls` edges:
- alpha -> beta
- multiply -> add (method-to-method inside impl Calculator; src MUST be the
  method chunk, not the impl container — spec A3 innermost attribution)
- test_alpha -> alpha (inside #[cfg(test)] mod; also the F-B test_of source)
- generic_probe::<i32>() and println! must produce NO edges
