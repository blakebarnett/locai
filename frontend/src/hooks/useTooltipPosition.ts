import { useEffect, useState, useRef } from 'react';

interface TooltipPosition {
  position: 'top' | 'bottom' | 'left' | 'right';
  transform: string;
  top?: string;
  bottom?: string;
  left?: string;
  right?: string;
}

export const useTooltipPosition = (isVisible: boolean) => {
  const [position, setPosition] = useState<TooltipPosition>({
    position: 'top',
    transform: 'translateX(-50%)',
    bottom: '100%',
    left: '50%',
  });
  
  const nodeRef = useRef<HTMLDivElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isVisible || !nodeRef.current || !tooltipRef.current) return;

    const updatePosition = () => {
      const node = nodeRef.current;
      const tooltip = tooltipRef.current;
      if (!node || !tooltip) return;

      const nodeRect = node.getBoundingClientRect();
      const tooltipRect = tooltip.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      // Calculate available space in each direction
      const spaceAbove = nodeRect.top;
      const spaceBelow = viewportHeight - nodeRect.bottom;
      const spaceLeft = nodeRect.left;
      const spaceRight = viewportWidth - nodeRect.right;

      // Tooltip dimensions
      const tooltipWidth = tooltipRect.width || 300;
      const tooltipHeight = tooltipRect.height || 200;

      let newPosition: TooltipPosition;

      // Prefer top, but switch to bottom if not enough space
      if (spaceAbove >= tooltipHeight + 10) {
        // Position above
        newPosition = {
          position: 'top',
          bottom: '100%',
          left: '50%',
          transform: 'translateX(-50%)',
        };

        // Adjust horizontal position if tooltip would go off-screen
        const tooltipLeft = nodeRect.left + nodeRect.width / 2 - tooltipWidth / 2;
        if (tooltipLeft < 10) {
          newPosition.left = '0';
          newPosition.transform = 'translateX(0)';
        } else if (tooltipLeft + tooltipWidth > viewportWidth - 10) {
          newPosition.left = '100%';
          newPosition.transform = 'translateX(-100%)';
        }
      } else if (spaceBelow >= tooltipHeight + 10) {
        // Position below
        newPosition = {
          position: 'bottom',
          top: '100%',
          left: '50%',
          transform: 'translateX(-50%)',
        };

        // Adjust horizontal position if tooltip would go off-screen
        const tooltipLeft = nodeRect.left + nodeRect.width / 2 - tooltipWidth / 2;
        if (tooltipLeft < 10) {
          newPosition.left = '0';
          newPosition.transform = 'translateX(0)';
        } else if (tooltipLeft + tooltipWidth > viewportWidth - 10) {
          newPosition.left = '100%';
          newPosition.transform = 'translateX(-100%)';
        }
      } else if (spaceRight >= tooltipWidth + 10) {
        // Position to the right
        newPosition = {
          position: 'right',
          left: '100%',
          top: '50%',
          transform: 'translateY(-50%)',
        };
      } else if (spaceLeft >= tooltipWidth + 10) {
        // Position to the left
        newPosition = {
          position: 'left',
          right: '100%',
          top: '50%',
          transform: 'translateY(-50%)',
        };
      } else {
        // Fallback: position below with adjusted horizontal position
        newPosition = {
          position: 'bottom',
          top: '100%',
          left: '10px',
          transform: 'translateX(0)',
        };
      }

      setPosition(newPosition);
    };

    // Update position immediately and on scroll/resize
    updatePosition();
    
    const handleUpdate = () => {
      requestAnimationFrame(updatePosition);
    };

    window.addEventListener('scroll', handleUpdate, true);
    window.addEventListener('resize', handleUpdate);

    return () => {
      window.removeEventListener('scroll', handleUpdate, true);
      window.removeEventListener('resize', handleUpdate);
    };
  }, [isVisible]);

  return { position, nodeRef, tooltipRef };
}; 