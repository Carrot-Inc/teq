// Program 497 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 852, "named, func arg types,
// braces"): scalac 3.8.4's parser accepts it, teq's reports "expected ':', found ')'". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 78
//     Defn.Object 0 77
//       Term.Name 7 11
//       Template 12 77
//         Template.Body 12 77
//           Defn.Given 14 74
//             Term.Name 20 23
//             Member.ParamClauseGroup 25 41
//               Type.ParamClause 25 25
//               Term.ParamClause 25 28
//                 Term.Param 26 27
//                   Name.Anonymous 26 26
//                   Type.Name 26 27
//               Term.ParamClause 32 38
//                 Term.Param 32 38
//                   Name.Anonymous 32 32
//                   Type.Apply 32 38
//                     Type.Name 32 35
//                     Type.ArgClause 35 38
//                       Type.Name 36 37
//             Template 42 74
//               Init 42 54
//                 Type.Apply 42 54
//                   Type.Name 42 45
//                   Type.ArgClause 45 54
//                     Type.Apply 46 53
//                       Type.Name 46 50
//                       Type.ArgClause 50 53
//                         Type.Name 51 52
//                 Name.Anonymous 55 55
//               Template.Body 55 74
//                 Defn.Def 59 72
//                   Term.Name 63 66
//                   Term.Name 69 72
object W497 {
given ord: (A) => Ord[A] => Ord[List[A]] {
  def foo = ???
}

}
