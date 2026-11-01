<script>
let options: Signal<Vec<String>> = Signal::new(
    [
    "Alabama",
    "Alaska",
    "American Samoa",
    "Arizona",
    "Arkansas",
    "California",
    "Colorado",
    "Connecticut",
    "Delaware",
    "District of Columbia",
    "Florida",
    "Georgia",
    "Guam",
    "Hawaii",
    "Idaho",
    "Illinois",
    "Indiana",
    "Iowa",
    "Kansas",
    "Kentucky",
    "Louisiana",
    "Maine",
    "Maryland",
    "Massachusetts",
    "Michigan",
    "Minnesota",
    "Mississippi",
    "Missouri",
    "Montana",
    "Nebraska",
    "Nevada",
    "New Hampshire",
    "New Jersey",
    "New Mexico",
    "New York",
    "North Carolina",
    "North Dakota",
    "Northern Marianas Islands",
    "Ohio",
    "Oklahoma",
    "Oregon",
    "Pennsylvania",
    "Puerto Rico",
    "Rhode Island",
    "South Carolina",
    "South Dakota",
    "Tennessee",
    "Texas",
    "Utah",
    "Vermont",
    "Virginia",
    "Virgin Islands",
    "Washington",
    "West Virginia",
    "Wisconsin",
    "Wyoming",
    ]
    .iter()
    .map(|name| (*name).to_owned())
    .collect(),
);
let value: Signal<String> = Signal::new(String::new());
</script>

<style>
/* viewport 640x400 */
/* vapors' combobox component (vapors-components) with APG's example's
   options, played by combobox.events against APG's own example
   (apg/combobox-autocomplete-list.html) in Chromium: only the a11y
   projection, focus, values and selections are compared, not the look. */
body { margin: 0; display: flex; flex-direction: column; font-size: 14px; }
input { width: 150px; height: 24px; }
.combobox-list { width: 168px; }
</style>

<vapors_components::combobox::Combobox options={options} bind:value={value} id="cb1-input" label="State" list_label="States" />
