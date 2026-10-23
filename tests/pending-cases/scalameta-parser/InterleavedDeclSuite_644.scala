// Program 644 of tests/fix/scalameta, from scalameta 4.17.4's InterleavedDeclSuite (line 106, "f[A](a: A, as:
// A*)[B](b: B, bs: B*)[C](implicit c: C): B [run with breaks before LP]"): scalac 3.8.4's parser reads it as
// scalameta does, `f[A](a1, a2)[B](b1, b2, b3)[C]`, and teq's otherwise. The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 54
//     Defn.Object 0 53
//       Term.Name 7 11
//       Template 12 53
//         Template.Body 12 53
//           Term.ApplyType 14 50
//             Term.Apply 14 47
//               Term.ApplyType 14 32
//                 Term.Apply 14 29
//                   Term.ApplyType 14 18
//                     Term.Name 14 15
//                     Type.ArgClause 15 18
//                       Type.Name 16 17
//                   Term.ArgClause 21 29
//                     Term.Name 22 24
//                     Term.Name 26 28
//                 Type.ArgClause 29 32
//                   Type.Name 30 31
//               Term.ArgClause 35 47
//                 Term.Name 36 38
//                 Term.Name 40 42
//                 Term.Name 44 46
//             Type.ArgClause 47 50
//               Type.Name 48 49
object W644 {
f[A]
  (a1, a2)[B]
  (b1, b2, b3)[C]

}
