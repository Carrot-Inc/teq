// Program 1091 of tests/fix/scalameta, from scalameta 4.17.4's TemplateSuite (line 42, "blank-after-template"):
// scalac 3.8.4's parser accepts it, teq's reports "expected an expression, found '['". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 50
//     Defn.Object 0 49
//       Term.Name 7 12
//       Template 13 49
//         Template.Body 13 49
//           Defn.Trait 16 46
//             Type.Name 22 23
//             Type.ParamClause 24 24
//             Ctor.Primary 24 24
//               Name.Anonymous 24 24
//             Template 24 46
//               Init 32 46
//                 Type.Apply 32 46
//                   Type.Name 32 33
//                   Type.ArgClause 36 46
//                     Type.Name 42 45
//                 Name.Anonymous 46 46
//               Template.Body 46 46
object W1091 {

trait B extends F
  [
    Int]

}
