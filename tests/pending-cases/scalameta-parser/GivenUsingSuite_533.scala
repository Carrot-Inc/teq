// Program 533 of tests/fix/scalameta, from scalameta 4.17.4's GivenUsingSuite (line 312, "given-with-empty-
// refinement"): scalac 3.8.4's parser accepts it, teq's reports "expected an identifier, found '{'". The expectation
// is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 82
//     Defn.Object 0 81
//       Term.Name 7 11
//       Template 12 81
//         Template.Body 12 81
//           Defn.Def 16 79
//             Term.Name 20 21
//             Term.Block 24 79
//               Defn.Given 26 76
//                 Name.Anonymous 32 32
//                 Template 32 76
//                   Init 32 34
//                     Type.Refine 32 34
//                       Stat.Block 32 34
//                     Name.Anonymous 35 35
//                   Template.Body 42 76
//                     Defn.ExtensionGroup 42 76
//                       Member.ParamClauseGroup 52 61
//                         Type.ParamClause 52 55
//                           Type.Param 53 54
//                             Type.Name 53 54
//                             Type.ParamClause 54 54
//                             Type.Bounds 54 54
//                         Term.ParamClause 55 61
//                           Term.Param 56 60
//                             Term.Name 56 57
//                             Type.Name 59 60
//                       Defn.Def 62 76
//                         Term.Name 66 71
//                         Lit.String 74 76
object W533 {
  def w = {
given {} with
  extension [T](t: T) def hello = ""

}
}
