// Overrides the final forwarder, which javac refuses.
public class FinalClient extends finalclone.E {
    @Override public String clone() { return "overridden"; }
    public static void main(String[] args) { System.out.println(new FinalClient().clone()); }
}
