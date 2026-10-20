export const marketingButtonClasses = `
  marketing-button [.marketing-page_&]:rounded-full [.marketing-page_&]:bg-[#222a25]
  [.marketing-page_&]:text-[#fff] [.marketing-page_&]:text-[13px] [.marketing-page_&]:h-10.5
  [.marketing-page_&]:p-[0_21px] [.marketing-page_&]:shadow-none [.marketing-page_&:hover]:bg-[#3d4a40]
  [.marketing-page_&:hover]:[transform:translateY(-1px)] [.platform-grid_&]:h-9
  [.platform-grid_&]:text-[10px] [.platform-grid_&]:px-3
  max-[700px]:[.marketing-nav-actions_&]:text-[10px]! max-[700px]:[.marketing-nav-actions_&]:h-[35px]!
  max-[700px]:[.marketing-nav-actions_&]:p-[0_13px]! max-[700px]:[.platform-grid_&]:p-[0_8px]
  max-[700px]:[.platform-grid_&]:text-[9px] max-[700px]:[.platform-grid_&_svg]:w-[11px]
`;

export const marketingPageClasses = `
  marketing-page bg-[#fcfcfa] text-[#202521] text-[15px] leading-[1.6] min-h-[100dvh] p-[16px_16px_0]
  [&_a]:no-underline [&_a]:text-[inherit] [&_button]:[-webkit-tap-highlight-color:transparent]
  [&_a]:[-webkit-tap-highlight-color:transparent] [&_summary]:[-webkit-tap-highlight-color:transparent]
  [&_a:focus-visible]:[outline:2px_solid_#287ca0] [&_a:focus-visible]:[outline-offset:5px]
  [&_summary:focus-visible]:[outline:2px_solid_#287ca0] [&_summary:focus-visible]:[outline-offset:5px]
  [&_h1]:m-[24px_0_22px] [&_:where(h2)]:text-[clamp(34px,_4vw,_54px)] [&_h2]:leading-[1.12]
  [&_h2]:font-medium [&_h2]:tracking-[-2.2px] [&_h3]:font-medium [&_h1]:text-[clamp(54px,_6.2vw,_88px)]
  [&_h1]:font-medium [&_h1]:tracking-[-5px] [&_h1]:leading-[1.03] max-[700px]:p-[8px_8px_0]
  max-[700px]:[&_h1]:tracking-[-3px] max-[700px]:[&_h1]:text-[64px]
  max-[700px]:[&_h2]:tracking-[-1.4px] motion-reduce:[&_*]:[transition:none]!
  motion-reduce:[&_*]:[animation:none]! motion-reduce:[&_*]:[scroll-behavior:auto]!
  motion-reduce:[&_*::before]:[transition:none]! motion-reduce:[&_*::before]:[animation:none]!
  motion-reduce:[&_*::before]:[scroll-behavior:auto]! motion-reduce:[&_*::after]:[transition:none]!
  motion-reduce:[&_*::after]:[animation:none]! motion-reduce:[&_*::after]:[scroll-behavior:auto]!
`;
