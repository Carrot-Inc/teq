// Program 722 of tests/fix/scalameta, from scalameta 4.17.4's MacroSuite (line 339, "#3610 3"): scalac 3.8.4's
// parser accepts it, teq's reports "expected an identifier, found 'type'". The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 69
//     Defn.Object 0 68
//       Term.Name 7 11
//       Template 12 68
//         Template.Body 12 68
//           Defn.Def 16 66
//             Term.Name 20 21
//             Term.Block 24 66
//               Term.QuotedMacroType 26 64
//                 Type.Block 29 62
//                   Decl.Type 29 47
//                     Type.Name 34 38
//                     Type.ParamClause 39 39
//                     Type.Bounds 39 47
//                       Type.Name 42 47
//                   Type.Apply 49 62
//                     Type.Name 49 51
//                     Type.ArgClause 51 62
//                       Type.Name 52 55
//                       Type.Name 57 61
object W722 {
  def w = {
'[ type tail <: Tuple; *:[Int, tail] ]
}
}
