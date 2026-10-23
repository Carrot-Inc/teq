// Program 616 of tests/fix/scalameta, from scalameta 4.17.4's InlineSuite (line 118, "inline-soft-ident"): scalac
// 3.8.4's parser accepts it, teq's reports "`inline` must be followed by an `if` or a `match`". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 27
//     Defn.Object 0 26
//       Term.Name 7 11
//       Template 12 26
//         Template.Body 12 26
//           Term.Apply 14 24
//             Term.Name 14 22
//             Term.ArgClause 22 24
object W616 {
`inline`()
}
