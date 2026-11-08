pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators);
pragma Restrictions (No_Exception_Propagation);

with Interfaces; use Interfaces;

package Wire_C1_Frame_Spark
  with Pure, SPARK_Mode => On
is
   pragma Elaborate_Body;

   Wire_Magic_C1 : constant Unsigned_16 := 16#8B15#;
   Kind_State_0_Cochain  : constant Unsigned_8 := 16#01#;
   Kind_Soldering_1_Form : constant Unsigned_8 := 16#02#;
   Kind_Torsion_2_Form   : constant Unsigned_8 := 16#03#;

   -- Validates the raw u8 frame kind without enum UB
   function Is_Valid_Soldering_Kind (Raw_Kind : Unsigned_8) return Boolean is
     (Raw_Kind = Kind_Soldering_1_Form)
     with Inline;

   -- Validates the 64-byte wire frame header fields
   function Is_Valid_Wire_C1_Header
     (Magic        : Unsigned_16;
      Raw_Kind     : Unsigned_8;
      Packet_Bytes : Natural;
      Addr_Mod_8   : Natural) return Boolean is
     (Packet_Bytes >= 64
      and then Addr_Mod_8 = 0
      and then Magic = Wire_Magic_C1
      and then Is_Valid_Soldering_Kind (Raw_Kind))
     with Inline;

   -- Idempotent causal clock transition guard:
   -- Returns True iff Incoming_Clock advances Current_Clock by exactly 1.
   function Should_Apply_Causal_Delta
     (Current_Clock  : Unsigned_32;
      Incoming_Clock : Unsigned_32) return Boolean is
     (Incoming_Clock = Current_Clock + 1)
     with Inline,
          Post => (if Should_Apply_Causal_Delta'Result then
                     not Should_Apply_Causal_Delta (Incoming_Clock, Incoming_Clock));

end Wire_C1_Frame_Spark;
