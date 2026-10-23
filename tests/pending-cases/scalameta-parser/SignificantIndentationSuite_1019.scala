// Program 1019 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 1287,
// "indented-apply"): scalac 3.8.4's parser reads it as scalameta does, `def method = fun(a, b, c)(d, e)`, and teq's
// otherwise. The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a
// line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 56
//     Defn.Object 0 55
//       Term.Name 7 12
//       Template 13 55
//         Template.Body 13 55
//           Defn.Def 15 52
//             Term.Name 19 25
//             Term.Apply 31 52
//               Term.Apply 31 41
//                 Term.Name 31 34
//                 Term.ArgClause 34 41
//                   Term.Name 35 36
//                   Term.Name 37 38
//                   Term.Name 39 40
//               Term.ArgClause 46 52
//                 Term.Name 47 48
//                 Term.Name 50 51
object W1019 {
def method = 
  fun(a,b,c)
    (d, e)

}
