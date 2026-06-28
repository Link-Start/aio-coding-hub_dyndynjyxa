import { HostRenderedContribution } from "./HostRenderedContribution";
import { useContributionsForSlot } from "./useActiveContributions";
import type { ContributionSlotProps } from "./types";

export function ContributionSlot({
  slotId,
  valuesByContributionId = {},
  onChange,
  onCommand,
  disabled,
}: ContributionSlotProps) {
  const { contributions } = useContributionsForSlot(slotId);

  if (contributions.length === 0) return null;

  return (
    <>
      {contributions.map((contribution) => (
        <HostRenderedContribution
          key={`${contribution.pluginId}:${contribution.contributionId}`}
          contribution={contribution}
          values={valuesByContributionId[contribution.contributionId] ?? {}}
          onChange={(fieldKey, value) => onChange?.(contribution, fieldKey, value)}
          onCommand={onCommand}
          disabled={disabled}
        />
      ))}
    </>
  );
}
