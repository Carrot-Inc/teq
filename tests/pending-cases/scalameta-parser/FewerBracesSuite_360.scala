// Program 360 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 1105, "scalafmt #3720 3
// complex, space after op, object indented"): scalac 3.8.4's parser reads it as scalameta does, `module object a {
// foo.bar(arg) ++ baz(arg) ++ qux`, and teq's otherwise. The expectation is scalameta's tree under dialects.Scala3,
// which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the program's first line
// below.
//   Source 0 82
//     Defn.Object 0 81
//       Term.Name 7 11
//       Template 12 81
//         Template.Body 12 81
//           Defn.Object 15 78
//             Term.Name 22 23
//             Template 23 78
//               Template.Body 23 78
//                 Term.ApplyInfix 27 78
//                   Term.ApplyInfix 27 67
//                     Term.Apply 27 43
//                       Term.Select 27 34
//                         Term.Name 27 30
//                         Term.Name 31 34
//                       Term.ArgClause 34 43
//                         Term.Block 34 43
//                           Term.Name 40 43
//                     Term.Name 46 48
//                     Type.ArgClause 53 53
//                     Term.ArgClause 53 67
//                       Term.Apply 53 67
//                         Term.Name 53 56
//                         Term.ArgClause 56 67
//                           Term.Block 56 67
//                             Term.Name 64 67
//                   Term.Name 72 74
//                   Type.ArgClause 75 75
//                   Term.ArgClause 75 78
//                     Term.Name 75 78
object W360 {

object a:
  foo.bar:
    arg
  ++
    baz:
      arg
    ++ qux

}
