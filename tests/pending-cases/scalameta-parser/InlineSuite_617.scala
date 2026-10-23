// Program 617 of tests/fix/scalameta, from scalameta 4.17.4's InlineSuite (line 127, "inline-soft-ident"): scalac
// 3.8.4's parser accepts it, teq's reports "`inline` must be followed by an `if` or a `match`". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 42
//     Defn.Object 0 41
//       Term.Name 7 11
//       Template 12 41
//         Template.Body 12 41
//           Defn.Object 14 39
//             Term.Name 21 22
//             Template 23 39
//               Template.Body 23 39
//                 Term.ApplyInfix 25 37
//                   Term.Name 25 33
//                   Term.Name 34 35
//                   Type.ArgClause 36 36
//                   Term.ArgClause 36 37
//                     Lit.Int 36 37
object W617 {
object X { `inline` + 3 }
}
