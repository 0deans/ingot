import i18n from "i18next"
import { memo, useMemo } from "react"
import { Badge } from "@/components/ui/badge"
import {
	Combobox,
	ComboboxContent,
	ComboboxEmpty,
	ComboboxInput,
	ComboboxItem,
	ComboboxList,
} from "@/components/ui/combobox"

export interface SearchableSelectOption {
	value: string
	label: string
	badge?: string
}

interface SearchableSelectProps {
	id?: string
	value: string
	onValueChange: (value: string) => void
	options: SearchableSelectOption[]
	placeholder?: string
	/** Shown in the input while the list is open (the input is also the search field) */
	searchPlaceholder?: string
	disabled?: boolean
	className?: string
}

/** A select you can type into to filter, built on shadcn's Combobox */
export const SearchableSelect = memo(
	({
		id,
		value,
		onValueChange,
		options,
		placeholder = i18n.t("select.placeholder"),
		searchPlaceholder,
		disabled = false,
		className,
	}: SearchableSelectProps) => {
		const selected = useMemo(() => options.find((o) => o.value === value) ?? null, [options, value])

		return (
			<Combobox
				items={options}
				value={selected}
				onValueChange={(option) => option && onValueChange(option.value)}
				itemToStringLabel={(option) => option.label}
				isItemEqualToValue={(a, b) => a.value === b.value}
				disabled={disabled}
			>
				<ComboboxInput
					id={id}
					placeholder={selected ? (searchPlaceholder ?? placeholder) : placeholder}
					disabled={disabled}
					className={className}
				/>
				<ComboboxContent>
					<ComboboxEmpty>{i18n.t("select.noMatches")}</ComboboxEmpty>
					<ComboboxList>
						{(option: SearchableSelectOption) => (
							<ComboboxItem key={option.value} value={option}>
								<span className="truncate">{option.label}</span>
								{option.badge && (
									<Badge variant="secondary" className="ml-auto font-mono">
										{option.badge}
									</Badge>
								)}
							</ComboboxItem>
						)}
					</ComboboxList>
				</ComboboxContent>
			</Combobox>
		)
	},
)

SearchableSelect.displayName = "SearchableSelect"

export default SearchableSelect
