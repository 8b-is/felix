-- wire_frame.ads

package Wire_Frame is

   -- Rational type representing p/q (reduced)
   type Rational is record
      Num : Integer;
      Den : Positive;
   end record;

   -- Invariants: Den > 0

   -- Wire C1 Frame record
   type Wire_C1_Frame is record
      Root_Weight : Rational;
      -- other fields omitted for brevity
   end record;

   -- Ingest a weight into the frame
   procedure Ingest (F : in out Wire_C1_Frame; W : Rational) with
      Pre   => W.Den > 0,
      Post  => F.Root_Weight = W;

end Wire_Frame;