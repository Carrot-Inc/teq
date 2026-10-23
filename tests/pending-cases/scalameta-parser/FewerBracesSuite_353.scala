// Program 353 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 876, "scalafmt #3720 2 complex,
// break after op, top-level, not indented"): scalac 3.8.4's parser accepts it, teq's reports "expected end of
// statement, found identifier". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree
// answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 72
//     Defn.Object 0 71
//       Term.Name 7 11
//       Template 11 71
//         Template.Body 11 71
//           Term.ApplyInfix 16 71
//             Term.ApplyInfix 16 56
//               Term.Apply 16 32
//                 Term.Select 16 23
//                   Term.Name 16 19
//                   Term.Name 20 23
//                 Term.ArgClause 23 32
//                   Term.Block 23 32
//                     Term.Name 29 32
//               Term.Name 35 37
//               Type.ArgClause 42 42
//               Term.ArgClause 42 56
//                 Term.Apply 42 56
//                   Term.Name 42 45
//                   Term.ArgClause 45 56
//                     Term.Block 45 56
//                       Term.Name 53 56
//             Term.Name 61 63
//             Type.ArgClause 68 68
//             Term.ArgClause 68 71
//               Term.Name 68 71
object W353:

  foo.bar:
    arg
  ++
    baz:
      arg
    ++
    qux
