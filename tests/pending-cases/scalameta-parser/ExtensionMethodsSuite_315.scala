// Program 315 of tests/fix/scalameta, from scalameta 4.17.4's ExtensionMethodsSuite (line 269, "method-multi-
// using"): scalac 3.8.4's parser accepts it, teq's reports "expected ',' or ')', found identifier". The expectation
// is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 146
//     Defn.Object 0 145
//       Term.Name 7 11
//       Template 12 145
//         Template.Body 12 145
//           Defn.Def 16 143
//             Term.Name 20 21
//             Term.Block 24 143
//               Defn.ExtensionGroup 26 141
//                 Member.ParamClauseGroup 39 108
//                   Type.ParamClause 39 39
//                   Term.ParamClause 39 61
//                     Term.Param 51 57
//                       Mod.Using 45 50
//                       Term.Name 51 52
//                       Type.Name 54 57
//                     Mod.Using 45 50
//                   Term.ParamClause 64 80
//                     Term.Param 70 76
//                       Term.Name 70 71
//                       Type.Name 73 76
//                   Term.ParamClause 83 108
//                     Term.Param 95 104
//                       Mod.Using 89 94
//                       Term.Name 95 96
//                       Type.Name 98 104
//                     Mod.Using 89 94
//                 Defn.Def 114 141
//                   Term.Name 118 123
//                   Term.ApplyInfix 126 141
//                     Term.ApplyInfix 126 131
//                       Term.Name 126 127
//                       Term.Name 128 129
//                       Type.ArgClause 130 130
//                       Term.ArgClause 130 131
//                         Term.Name 130 131
//                     Term.Name 132 133
//                     Type.ArgClause 134 134
//                     Term.ArgClause 134 141
//                       Term.Select 134 141
//                         Term.Name 134 135
//                         Term.Name 136 141
object W315 {
  def w = {
extension 
  (
    using a: Int
  )
  (
    b: Int
  )
  (
    using c: String
  ) 
    def hello = a + b + c.toInt
}
}
