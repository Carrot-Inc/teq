// Program 298 of tests/fix/scalameta, from scalameta 4.17.4's ExportImportSuite (line 100, "#3754 export
// Type.this.field"): scalac 3.8.4's parser accepts it, teq's reports "expected an identifier, found 'this'". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 55
//     Defn.Object 0 54
//       Term.Name 7 11
//       Template 12 54
//         Template.Body 12 54
//           Defn.Class 14 51
//             Mod.Case 14 18
//             Type.Name 25 26
//             Type.ParamClause 26 26
//             Ctor.Primary 26 28
//               Name.Anonymous 26 26
//               Term.ParamClause 26 28
//             Template 28 51
//               Template.Body 28 51
//                 Export 32 51
//                   Importer 39 51
//                     Term.This 39 45
//                       Name.Indeterminate 39 40
//                     Importee.Name 46 51
//                       Name.Indeterminate 46 51
object W298 {
case class A():
  export A.this.value

}
