// Program 1030 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 1555,
// "indented-double-new"): scalac 3.8.4's parser reads it as scalameta does, `new fun(a, b, c)(d, e)(f, g)`, and
// teq's otherwise. The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 60
//     Defn.Object 0 59
//       Term.Name 7 12
//       Template 13 59
//         Template.Body 13 59
//           Term.New 15 56
//             Init 19 56
//               Type.Name 19 22
//               Name.Anonymous 27 27
//               Term.ArgClause 27 34
//                 Term.Name 28 29
//                 Term.Name 30 31
//                 Term.Name 32 33
//               Term.ArgClause 39 45
//                 Term.Name 40 41
//                 Term.Name 43 44
//               Term.ArgClause 50 56
//                 Term.Name 51 52
//                 Term.Name 54 55
object W1030 {
new fun
    (a,b,c)
    (d, e)
    (f, g)

}
