// Program 751 of tests/fix/scalameta, from scalameta 4.17.4's MinorDottySuite (line 83, "case-classes-empty-plist"):
// scalac 3.8.4's parser accepts it. The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree is to answer to: a node a line, its
// start and end in UTF-16 units from the program's first line below.
//   Source 0 46
//     Defn.Object 0 45
//       Term.Name 7 11
//       Template 12 45
//         Template.Body 12 45
//           Defn.Class 14 43
//             Mod.Case 14 18
//             Type.Name 25 26
//             Type.ParamClause 27 27
//             Ctor.Primary 27 43
//               Mod.Annot 27 40
//                 Init 28 40
//                   Type.Name 28 38
//                   Name.Anonymous 38 38
//                   Term.ArgClause 38 40
//               Name.Anonymous 41 41
//               Term.ParamClause 41 43
//             Template 43 43
//               Template.Body 43 43
object W751 {
case class A @deprecated() ()
}
