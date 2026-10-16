// Calls the diamond's clone by its own signature and by `Root`'s, its bridge.
public class DiamondJava {
    public static void main(String[] args) {
        clonediamond.Diamond d = new clonediamond.Diamond();
        System.out.println(d.clone());
        System.out.println(((clonediamond.Root) d).clone());
    }
}
