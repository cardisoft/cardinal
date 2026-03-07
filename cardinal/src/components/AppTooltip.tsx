import React from 'react';
import { Tooltip } from 'react-tooltip';
import 'react-tooltip/dist/react-tooltip.css';
import { APP_TOOLTIP_ID } from '../utils/tooltip';

export function AppTooltip(): React.JSX.Element {
  return (
    <Tooltip
      id={APP_TOOLTIP_ID}
      className="app-tooltip"
      opacity={1}
      place="bottom"
      delayShow={250}
    />
  );
}
