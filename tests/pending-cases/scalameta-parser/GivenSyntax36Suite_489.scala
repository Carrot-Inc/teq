// Program 489 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 710, "context a poly
// function, braces"): scalac 3.8.4's parser accepts it, teq's reports "expected an identifier, found '['". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 98
//     Defn.Object 0 97
//       Term.Name 7 11
//       Template 12 97
//         Template.Body 12 97
//           Defn.Given 14 94
//             Term.Name 20 23
//             Member.ParamClauseGroup 25 61
//               Type.ParamClause 25 25
//               Term.ParamClause 25 58
//                 Term.Param 26 57
//                   Name.Anonymous 26 26
//                   Type.PolyFunction 26 57
//                     Type.ParamClause 26 29
//                       Type.Param 27 28
//                         Type.Name 27 28
//                         Type.ParamClause 28 28
//                         Type.Bounds 28 28
//                     Type.Function 33 57
//                       Type.FuncParamClause 33 41
//                         Type.Apply 34 40
//                           Type.Name 34 37
//                           Type.ArgClause 37 40
//                             Type.Name 38 39
//                       Type.Apply 45 57
//                         Type.Name 45 48
//                         Type.ArgClause 48 57
//                           Type.Apply 49 56
//                             Type.Name 49 53
//                             Type.ArgClause 53 56
//                               Type.Name 54 55
//             Template 62 94
//               Init 62 74
//                 Type.Apply 62 74
//                   Type.Name 62 65
//                   Type.ArgClause 65 74
//                     Type.Apply 66 73
//                       Type.Name 66 70
//                       Type.ArgClause 70 73
//                         Type.Name 71 72
//                 Name.Anonymous 75 75
//               Template.Body 75 94
//                 Defn.Def 79 92
//                   Term.Name 83 86
//                   Term.Name 89 92
object W489 {
given ord: ([A] => (Ord[A]) => Ord[List[A]]) => Ord[List[A]] {
  def foo = ???
}

}
