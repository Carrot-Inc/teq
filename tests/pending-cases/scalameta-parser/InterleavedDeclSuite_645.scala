// Program 645 of tests/fix/scalameta, from scalameta 4.17.4's InterleavedDeclSuite (line 121, "f[A](a: A, as:
// A*)[B](b: B, bs: B*)[C](implicit c: C): B [run with breaks]"): scalac 3.8.4's parser accepts it, teq's reports
// "expected an expression, found '['". The expectation is scalameta's tree under dialects.Scala3, which teq fix's
// tree answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 63
//     Defn.Object 0 62
//       Term.Name 7 11
//       Template 12 62
//         Template.Body 12 62
//           Term.ApplyType 14 59
//             Term.Apply 14 53
//               Term.ApplyType 14 38
//                 Term.Apply 14 32
//                   Term.ApplyType 14 21
//                     Term.Name 14 15
//                     Type.ArgClause 18 21
//                       Type.Name 19 20
//                   Term.ArgClause 24 32
//                     Term.Name 25 27
//                     Term.Name 29 31
//                 Type.ArgClause 35 38
//                   Type.Name 36 37
//               Term.ArgClause 41 53
//                 Term.Name 42 44
//                 Term.Name 46 48
//                 Term.Name 50 52
//             Type.ArgClause 56 59
//               Type.Name 57 58
object W645 {
f
  [A]
  (a1, a2)
  [B]
  (b1, b2, b3)
  [C]

}
