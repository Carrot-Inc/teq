# The token matrix's probes (fix-speed's table, §6)

- `Meta.scala`: scalameta 4.17.3's Scala 3 dialect over a file, `raw` (its tokenizer) or `tree` (a parsed `Source`'s
  `tree.tokens`, what scalafix's rules read), one line a token, `<class>|<start>|<end>|<text>`, with the default
  `TokenizerOptions` (`Opts.scala` prints its `groupWhitespace`, false):
  `scala-cli run -S 3.8.4 --server=false Meta.scala -- Input.scala raw`.
- `Dump.scala` (with `dump.conf`): the same through teq's bridge, a `file:` rule linting every token:
  `teq compiler fix Input.scala --config dump.conf --check`. Joined by span, the two give the matrix.
- `Macro.scala`, `Range.scala`, `Quoted.scala`, `EndMarkers.scala`, `EndVal.scala`: what scalac 3.8.4
  (`scala-cli compile -S 3.8.4 --server=false`) and teq (`teq compiler check`) accept of `macro` as a name,
  out-of-range literals, quoted identifiers in a splice and end markers with keyword names.
