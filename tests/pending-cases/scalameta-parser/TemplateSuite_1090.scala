// Program 1090 of tests/fix/scalameta, from scalameta 4.17.4's TemplateSuite (line 41, "blank-after-template"):
// scalac 3.8.4's parser accepts it, teq's reports "expected an expression, found '['". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 45
//     Defn.Object 0 44
//       Term.Name 7 12
//       Template 13 44
//         Template.Body 13 44
//           Defn.Trait 16 41
//             Type.Name 22 23
//             Type.ParamClause 24 24
//             Ctor.Primary 24 24
//               Name.Anonymous 24 24
//             Template 24 41
//               Init 32 41
//                 Type.Apply 32 41
//                   Type.Name 32 33
//                   Type.ArgClause 36 41
//                     Type.Name 37 40
//                 Name.Anonymous 41 41
//               Template.Body 41 41
object W1090 {

trait B extends F
  [Int]

}
