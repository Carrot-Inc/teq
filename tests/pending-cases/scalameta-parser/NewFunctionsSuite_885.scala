// Program 885 of tests/fix/scalameta, from scalameta 4.17.4's NewFunctionsSuite (line 421, "type-lmabda-bounds"):
// scalac 3.8.4's parser accepts it, teq's reports "expected an identifier, found '['". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 86
//     Defn.Object 0 85
//       Term.Name 7 11
//       Template 12 85
//         Template.Body 12 85
//           Defn.Class 14 83
//             Mod.Abstract 14 22
//             Type.Name 29 39
//             Type.ParamClause 39 83
//               Type.Param 40 82
//                 Type.Name 40 41
//                 Type.ParamClause 41 44
//                   Type.Param 42 43
//                     Name.Placeholder 42 43
//                     Type.ParamClause 43 43
//                     Type.Bounds 43 43
//                 Type.Bounds 44 82
//                   Type.Lambda 46 82
//                     Type.ParamClause 46 52
//                       Type.Param 47 51
//                         Type.Name 47 48
//                         Type.ParamClause 48 51
//                           Type.Param 49 50
//                             Name.Placeholder 49 50
//                             Type.ParamClause 50 50
//                             Type.Bounds 50 50
//                         Type.Bounds 51 51
//                     Type.Apply 57 82
//                       Type.Name 57 68
//                       Type.ArgClause 68 82
//                         Type.Name 69 70
//                         Type.Name 72 81
//             Ctor.Primary 83 83
//               Name.Anonymous 83 83
//             Template 83 83
//               Template.Body 83 83
object W885 {
abstract class Repository[F[_]: [G[_]] =>> MonadCancel[G, Throwable]]
}
