pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces;           use Interfaces;
with Felix_Nand_TCB;       use Felix_Nand_TCB;
with Ljodatal_18_Spark;    use Ljodatal_18_Spark;
with Agp_Iommu_Tlb_Spark;  use Agp_Iommu_Tlb_Spark;

package M8_Commerce_Webhook_Spark
  with Pure, SPARK_Mode => On
is
   Max_Replay_Skew_Secs : constant Unsigned_64 := 300;
   Ticket_TTL_Secs      : constant Unsigned_64 := 3_600;

   -- Triadic Payment Gateway Poles: +1 = Stripe, 0 = Revolut, -1 = PayPal
   Pole_Stripe  : constant Balanced_Trit := 1;
   Pole_Revolut : constant Balanced_Trit := 0;
   Pole_PayPal  : constant Balanced_Trit := -1;

   pragma Assert (Integer (Pole_Stripe) + Integer (Pole_Revolut) + Integer (Pole_PayPal) = 0);

   type Webhook_Gate_Verdict is
     (Webhook_Verified_Mint,
      Webhook_Reject_Bad_HMAC,
      Webhook_Reject_Replay_Skew,
      Webhook_Reject_Zero_Amount,
      Webhook_Reject_Charm18_Seal);

   -- Evaluates the 5-Gate Payment Webhook Admission Check in SPARK
   function Verify_Payment_Webhook_Gate
     (HMAC_Matches       : Boolean;
      Webhook_Timestamp  : Unsigned_64;
      Server_Now_Unix    : Unsigned_64;
      Amount_Minor_Units : Unsigned_64;
      Requested_Caps     : Capability_Mask) return Webhook_Gate_Verdict
     with Inline,
          Pre  => True,
          Post => (if Verify_Payment_Webhook_Gate'Result = Webhook_Verified_Mint then
                     HMAC_Matches
                     and then Amount_Minor_Units > 0
                     and then (Requested_Caps and Cap_Root_Seal) = 0
                     and then (if Webhook_Timestamp >= Server_Now_Unix then
                                 Webhook_Timestamp - Server_Now_Unix <= Max_Replay_Skew_Secs
                               else
                                 Server_Now_Unix - Webhook_Timestamp <= Max_Replay_Skew_Secs));

   -- Mints a Read-Only (`Trit_Perm = 0`) Capability PTE for an `.m8` download ticket
   function Mint_ReadOnly_M8_Download_PTE
     (Gateway_Pole    : Balanced_Trit;
      Slot_ID         : Cap_Slot_Byte;
      Phys_Base_Block : Phys_Block_36;
      Blocks_Count    : Unsigned_32;
      Epoch_Gen       : Epoch_Gen_Byte) return Cap_PTE_Record
     with Pre  => Blocks_Count > 0,
          Post => Mint_ReadOnly_M8_Download_PTE'Result.Valid
                  and then Mint_ReadOnly_M8_Download_PTE'Result.Trit_Perm = 0
                  and then (Mint_ReadOnly_M8_Download_PTE'Result.Allowed_Caps and Cap_Root_Seal) = 0;

end M8_Commerce_Webhook_Spark;
