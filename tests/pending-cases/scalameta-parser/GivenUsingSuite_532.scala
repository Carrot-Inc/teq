// Program 532 of tests/fix/scalameta, from scalameta 4.17.4's GivenUsingSuite (line 284, "given-with-export"):
// scalac 3.8.4's parser accepts it, teq's reports "exports are only allowed from objects and classes, they can not
// belong to local blocks". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers
// to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 96
//     Defn.Object 0 95
//       Term.Name 7 11
//       Template 12 95
//         Template.Body 12 95
//           Defn.Def 16 93
//             Term.Name 20 21
//             Term.Block 24 93
//               Defn.Given 26 90
//                 Term.Name 32 38
//                 Template 40 90
//                   Init 40 48
//                     Type.Apply 40 48
//                       Type.Name 40 43
//                       Type.ArgClause 43 48
//                         Type.Name 44 47
//                     Name.Anonymous 49 49
//                   Template.Body 56 90
//                     Export 56 71
//                       Importer 63 71
//                         Term.Name 63 67
//                         Importee.Name 68 71
//                           Name.Indeterminate 68 71
//                     Defn.Def 74 90
//                       Term.Name 78 79
//                       Member.ParamClauseGroup 79 81
//                         Type.ParamClause 79 79
//                         Term.ParamClause 79 81
//                       Type.Name 83 86
//                       Lit.Int 89 90
object W532 {
  def w = {
given intOrd: Ord[Int] with
  export math.max
  def f(): Int = 1

}
}
