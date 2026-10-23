// Program 721 of tests/fix/scalameta, from scalameta 4.17.4's MacroSuite (line 330, "#3610 2"): scalac 3.8.4's
// parser accepts it, teq's reports "expected an identifier, found 'type'". The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 57
//     Defn.Object 0 56
//       Term.Name 7 11
//       Template 12 56
//         Template.Body 12 56
//           Defn.Def 16 54
//             Term.Name 20 21
//             Term.Block 24 54
//               Term.QuotedMacroType 26 52
//                 Type.Block 29 50
//                   Defn.Type 29 41
//                     Type.Name 34 35
//                     Type.ParamClause 36 36
//                     Type.Name 38 41
//                     Type.Bounds 36 36
//                   Type.Apply 43 50
//                     Type.Name 43 47
//                     Type.ArgClause 47 50
//                       Type.Name 48 49
object W721 {
  def w = {
'[ type t = Int; List[t] ]
}
}
